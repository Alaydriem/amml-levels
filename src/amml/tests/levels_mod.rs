use alaydriem_levels::LevelsMod;
use amm_client::link::LinkUses;
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amml_api::AmmlMod;
use amml_api::testing::{FakeCanvas, FakeClient, FakeWorld};

use crate::support::levels_rig::{BORDERS, LevelsRig, SPHERE_CLEAR, SPHERE_DROP, TOGGLE};

const MANIFEST: &str = include_str!("../amml.hcl");

// the number after `key=` in a logged line
fn field(line: &str, key: &str) -> usize {
    let start = line.find(&format!("{key}=")).unwrap() + key.len() + 1;
    line[start..].split(' ').next().unwrap().parse().unwrap()
}

#[test]
fn the_manifest_names_the_mod_and_lists_no_entries() {
    assert!(MANIFEST.contains("namespace    = \"alaydriem\""));
    assert!(MANIFEST.contains("name         = \"levels\""));
    assert!(MANIFEST.contains("api_version  = 3"));
    assert!(!MANIFEST.contains("entries"));
    assert!(!MANIFEST.contains("amml_version"));
}

#[test]
fn the_manifest_uses_block_is_the_uses_of_the_mod() {
    let manifest = MANIFEST.replace("\r\n", "\n");

    assert_eq!(manifest.matches(&LevelsMod::USES.to_hcl()).count(), 1);
    assert_eq!(LevelsMod::USES.write_into(&manifest), manifest);
}

#[test]
fn link_uses_are_complete() {
    assert_eq!(LinkUses::check(&LevelsMod::USES), Ok(()));
}

#[test]
fn the_first_tick_with_the_manager_links_it() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(LevelsRig::in_world());

    rig.tick();
    rig.tick();

    assert_eq!(rig.lines("info levels: manager linked"), ["info levels: manager linked"]);
}

#[test]
fn without_the_manager_the_overlay_stays_off() {
    let rig = LevelsRig::without_manager();
    rig.set_client(LevelsRig::in_world());
    FakeTable::mods_loaded(&rig.game);

    rig.tick();
    rig.press(TOGGLE);
    let canvas = rig.frame();

    assert_eq!(rig.lines("error levels: manager missing, overlay off").len(), 1);
    assert!(rig.lines("info levels: manager linked").is_empty());
    assert!(canvas.calls.borrow().is_empty());
    assert!(LevelsRig::acquired().is_empty());
}

#[test]
fn the_toggle_takes_the_lease_scans_and_draws_the_labels() {
    let rig = LevelsRig::turned_on();

    let canvas = rig.frame();

    assert_eq!(LevelsRig::acquired(), [Capability::WorldDraw]);
    assert_eq!(rig.lines("info levels: world draw lease held").len(), 1);
    let scan = &rig.lines("info levels: scan")[0];
    assert!(field(scan, "columns") > 0 && field(scan, "surfaces") > 0, "{scan}");
    let built = &rig.lines("info levels: mesh built slot=label")[0];
    let quads = field(built, "quads");
    assert!(quads > 0, "{built}");
    let calls = canvas.calls.borrow();
    assert_eq!(calls[0], format!("build quads={quads} style=alpha/1/0/2/full/white"));
    assert!(calls[1].starts_with("draw "), "{calls:?}");
}

#[test]
fn the_labels_are_built_once_and_drawn_every_frame() {
    let rig = LevelsRig::turned_on();

    let first = rig.frame();
    let second = rig.frame();

    assert_eq!(first.calls.borrow().iter().filter(|call| call.starts_with("build")).count(), 1);
    assert_eq!(second.calls.borrow().iter().filter(|call| call.starts_with("build")).count(), 0);
    assert_eq!(second.calls.borrow().iter().filter(|call| call.starts_with("draw")).count(), 1);
    assert_eq!(rig.lines("info levels: mesh built slot=label").len(), 1);
}

#[test]
fn the_border_key_builds_the_border_mesh() {
    let rig = LevelsRig::turned_on();
    rig.frame();

    rig.press(BORDERS);
    rig.frame();

    assert_eq!(rig.lines("info levels: mesh built slot=border").len(), 1);
}

#[test]
fn a_dropped_sphere_is_built_and_drawn_with_the_negated_spin() {
    let rig = LevelsRig::with_manager("spheres {\n  spin = -90.0\n}\n");
    rig.set_client(LevelsRig::in_world());
    rig.tick();
    rig.press(SPHERE_DROP);

    let canvas = rig.frame();

    assert_eq!(rig.lines("info levels: sphere dropped count=1").len(), 1);
    assert_eq!(rig.lines("info levels: mesh built slot=shell").len(), 1);
    let calls = canvas.calls.borrow();
    let draw = calls.iter().find(|call| call.starts_with("draw")).unwrap();
    let yaw: f32 = draw.rsplit("yaw=").next().unwrap().parse().unwrap();
    assert!(yaw > 0.0, "a spin of -90 degrees a second turns the loader's yaw the other way: {draw}");
}

#[test]
fn a_sphere_keeps_its_sub_block_position_in_the_draw() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(LevelsRig::in_world());
    rig.tick();
    rig.press(SPHERE_DROP);

    let canvas = rig.frame();

    let calls = canvas.calls.borrow();
    let draw = calls.iter().find(|call| call.starts_with("draw")).unwrap();
    let parts: Vec<&str> = draw.split(' ').collect();
    let (x, y, z): (f64, f64, f64) = (parts[2].parse().unwrap(), parts[3].parse().unwrap(), parts[4].parse().unwrap());
    assert_eq!((x, y, z), (10.5, 64.0, -19.5));
}

#[test]
fn clearing_the_spheres_is_logged_and_frees_the_shell_mesh() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(LevelsRig::in_world());
    rig.tick();
    rig.press(SPHERE_DROP);
    rig.frame();

    rig.press(SPHERE_CLEAR);
    let canvas = rig.frame();

    assert_eq!(rig.lines("info levels: spheres cleared").len(), 1);
    assert_eq!(canvas.frees.borrow().len(), 1);
    assert_eq!(rig.game.live_meshes(), 0);
}

#[test]
fn the_toggle_off_releases_the_lease_and_frees_the_meshes() {
    let rig = LevelsRig::turned_on();
    rig.frame();
    assert_eq!(rig.game.live_meshes(), 1);

    rig.press(TOGGLE);
    let canvas = rig.frame();

    assert_eq!(rig.lines("info levels: world draw lease released").len(), 1);
    assert_eq!(LevelsRig::released(), [Capability::WorldDraw]);
    assert_eq!(canvas.frees.borrow().len(), 1);
    assert!(canvas.calls.borrow().is_empty());
}

#[test]
fn a_graphics_mode_switch_rebuilds_the_labels_in_the_same_frame() {
    let rig = LevelsRig::turned_on();
    rig.frame();

    rig.game.switch_graphics();
    let canvas = rig.frame();

    assert_eq!(rig.lines("info levels: meshes stale, rebuilt slot=label").len(), 1);
    let calls = canvas.calls.borrow();
    assert!(calls[0].starts_with("build quads="), "{calls:?}");
    assert!(calls[1].starts_with("draw "), "{calls:?}");
}

#[test]
fn leaving_the_world_turns_the_overlay_off_and_a_new_world_builds_again() {
    let rig = LevelsRig::turned_on();
    rig.frame();

    rig.game.leave_mesh_world();
    rig.set_client(FakeClient::default());
    rig.tick();
    rig.set_client(LevelsRig::in_world());
    rig.tick();
    rig.press(TOGGLE);
    let canvas = rig.frame();

    assert_eq!(LevelsRig::released(), [Capability::WorldDraw]);
    assert_eq!(LevelsRig::acquired(), [Capability::WorldDraw, Capability::WorldDraw]);
    assert_eq!(rig.lines("info levels: mesh built slot=label").len(), 2);
    assert!(canvas.calls.borrow()[0].starts_with("build quads="));
    assert!(rig.lines("info levels: meshes stale, rebuilt slot=label").is_empty());
}

#[test]
fn a_hidden_hud_draws_no_labels() {
    let rig = LevelsRig::turned_on();
    rig.set_client(FakeClient { hide_hud: true, ..LevelsRig::in_world() });
    rig.tick();

    let canvas = rig.frame();

    assert!(canvas.calls.borrow().is_empty());
    assert!(rig.lines("info levels: mesh built slot=label").is_empty());
}

#[test]
fn an_open_menu_ignores_the_toggle() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(FakeClient { showing_menu: true, ..LevelsRig::in_world() });
    rig.tick();

    rig.press(TOGGLE);
    let canvas = rig.frame();

    assert!(LevelsRig::acquired().is_empty());
    assert!(canvas.calls.borrow().is_empty());
}

#[test]
fn a_dimension_change_drops_the_labels_without_a_stale_rebuild() {
    let rig = LevelsRig::turned_on();
    rig.frame();

    let empty_nether = FakeWorld { surfaces: Default::default(), ..LevelsRig::world() };
    rig.set_client(FakeClient { dimension: Some(1), world: Some(empty_nether), ..LevelsRig::in_world() });
    rig.tick();
    let canvas = rig.frame();

    assert_eq!(canvas.frees.borrow().len(), 1);
    assert!(rig.lines("info levels: meshes stale, rebuilt slot=label").is_empty());
}

#[test]
fn with_telemetry_on_the_draw_is_logged_with_the_graphics_mode() {
    let rig = LevelsRig::with_manager("telemetry = true\n");
    rig.set_client(LevelsRig::in_world());
    rig.tick();
    rig.press(TOGGLE);
    rig.tick();

    rig.render(&FakeCanvas { camera_position: Some(LevelsRig::FEET), vibrant: true, ..FakeCanvas::default() });

    assert_eq!(rig.lines("info levels: world drawn meshes=1 vibrant=1").len(), 1);
}

#[test]
fn without_telemetry_no_draw_line_is_logged() {
    let rig = LevelsRig::turned_on();

    rig.frame();

    assert!(rig.lines("info levels: world drawn").is_empty());
}

#[test]
fn a_world_render_before_the_manager_connects_draws_nothing() {
    let rig = LevelsRig::with_manager("");
    rig.set_client(LevelsRig::in_world());
    let canvas = FakeCanvas::default();

    rig.game.raise_world_rendered(&canvas);

    assert!(canvas.calls.borrow().is_empty());
}

#[test]
fn disable_while_on_releases_the_lease_before_the_link_closes() {
    let rig = LevelsRig::turned_on();
    rig.frame();

    rig.game.disable::<LevelsMod>();

    let calls = FakeTable::calls();
    let release = calls.iter().position(|call| *call == "release").unwrap();
    let unregister = calls.iter().position(|call| *call == "unregister").unwrap();
    assert!(release < unregister, "{calls:?}");
    assert_eq!(rig.lines("info levels: world draw lease released").len(), 1);
}

#[test]
fn an_unreadable_dimension_is_logged_once_per_change_of_state() {
    let rig = LevelsRig::turned_on();
    let unreadable = || FakeClient { dimension: None, ..LevelsRig::in_world() };
    let line = "warn levels: dimension unreadable";

    rig.set_client(unreadable());
    rig.tick();
    rig.tick();
    assert_eq!(rig.lines(line).len(), 1);

    rig.set_client(LevelsRig::in_world());
    rig.tick();
    assert_eq!(rig.lines(line).len(), 1);

    rig.set_client(unreadable());
    rig.tick();
    assert_eq!(rig.lines(line).len(), 2);
}

#[test]
fn a_revoked_lease_frees_the_meshes_and_the_next_frame_draws_nothing() {
    let rig = LevelsRig::turned_on();
    rig.frame();
    assert_eq!(rig.game.live_meshes(), 1);

    FakeTable::push_event(amm_core::structs::event::ModEvent::Revoked {
        capability: Capability::WorldDraw,
        by: amm_core::structs::registry::ModIndex(2),
    });
    rig.tick();
    let canvas = rig.frame();

    assert_eq!(canvas.frees.borrow().len(), 1);
    assert!(canvas.calls.borrow().is_empty());
    assert_eq!(rig.game.live_meshes(), 0);
}

#[test]
fn a_manager_that_is_gone_frees_the_meshes_and_the_next_frame_draws_nothing() {
    let rig = LevelsRig::turned_on();
    rig.frame();

    FakeTable::gone(&rig.game);
    rig.tick();
    let canvas = rig.frame();

    assert_eq!(canvas.frees.borrow().len(), 1);
    assert!(canvas.calls.borrow().is_empty());
    assert_eq!(rig.game.live_meshes(), 0);
}

#[test]
fn a_change_of_graphics_mode_is_logged_once() {
    let rig = LevelsRig::turned_on();

    rig.frame();
    rig.frame();
    rig.render(&FakeCanvas { camera_position: Some(LevelsRig::FEET), vibrant: true, ..FakeCanvas::default() });
    rig.render(&FakeCanvas { camera_position: Some(LevelsRig::FEET), vibrant: true, ..FakeCanvas::default() });

    assert_eq!(
        rig.lines("info levels: graphics mode seen"),
        ["info levels: graphics mode seen vibrant=0", "info levels: graphics mode seen vibrant=1"]
    );
}
