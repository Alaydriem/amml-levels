use levels_core::config::{InfoConfig, LabelConfig, LightConfig};
use levels_core::services::InfoPanel;
use levels_core::structs::frame::ColumnRequest;
use levels_core::structs::info::{ActorSample, InfoMode, InfoNames, InfoSample, InfoTarget};
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::manager::ManagerSignal;
use levels_core::structs::world::{BlockPos, ColumnPos, Vec3f};

const ZOMBIE: u32 = 0x20 | 0x100 | 0x800;
// a low byte with no entry in the kind name table
const UNNAMED_HOSTILE: u32 = 0x3C | 0x100 | 0x800;

struct Fixture;

impl Fixture {
    fn panel_with(config: InfoConfig) -> InfoPanel {
        InfoPanel::new(&config, LightConfig::default(), &LabelConfig::default())
    }

    fn panel() -> InfoPanel {
        Fixture::panel_with(InfoConfig::default())
    }

    fn shown() -> InfoPanel {
        let mut panel = Fixture::panel();
        panel.signal(ManagerSignal::InfoPressed);
        panel
    }

    fn vec3(x: f32, y: f32, z: f32) -> Vec3f {
        Vec3f { x, y, z }
    }

    fn sample_at(time_seconds: f64) -> InfoSample {
        InfoSample {
            time_seconds,
            position: Fixture::vec3(12.345, 64.0, -3.25),
            yaw: 180.0,
            pitch: 12.5,
            dimension: 0,
            biome: Some(7),
            sky_darken: 0,
            target: InfoTarget::None,
        }
    }

    fn names() -> InfoNames<'static> {
        InfoNames { biome: Some("deep_dark"), target_block: Some("minecraft:stone") }
    }

    fn surface(y: i32, block_light: u8, sky_light: u8) -> SurfaceSample {
        SurfaceSample { pos: BlockPos { x: 12, y, z: -4 }, block_light, sky_light }
    }

    fn line<'a>(panel: &'a InfoPanel, prefix: &str) -> Option<&'a str> {
        panel.lines().iter().map(|line| line.as_str()).find(|text| text.starts_with(prefix))
    }

    fn color_of(panel: &InfoPanel, prefix: &str) -> Option<[f32; 3]> {
        panel.lines().iter().find(|line| line.as_str().starts_with(prefix)).map(|line| line.color())
    }

    fn actor(min: Vec3f, max: Vec3f, actor_type: u32) -> ActorSample {
        ActorSample { min, max, actor_type, is_local_player: false }
    }
}

#[test]
fn toggle_shows_and_hides() {
    let mut panel = Fixture::panel();

    panel.signal(ManagerSignal::InfoPressed);
    assert!(panel.is_shown());
    panel.signal(ManagerSignal::InfoReleased);
    assert!(panel.is_shown());
    panel.signal(ManagerSignal::InfoPressed);
    assert!(!panel.is_shown());
}

#[test]
fn hold_shows_while_down() {
    let mut panel = Fixture::panel_with(InfoConfig { mode: InfoMode::Hold, ..InfoConfig::default() });

    panel.signal(ManagerSignal::InfoPressed);
    assert!(panel.is_shown());
    panel.signal(ManagerSignal::InfoReleased);
    assert!(!panel.is_shown());
}

#[test]
fn release_without_press_is_ignored() {
    let mut toggle = Fixture::panel();
    toggle.signal(ManagerSignal::InfoReleased);
    assert!(!toggle.is_shown());
    toggle.signal(ManagerSignal::InfoPressed);
    assert!(toggle.is_shown());

    let mut hold = Fixture::panel_with(InfoConfig { mode: InfoMode::Hold, ..InfoConfig::default() });
    hold.signal(ManagerSignal::InfoReleased);
    assert!(!hold.is_shown());
    hold.signal(ManagerSignal::InfoPressed);
    assert!(hold.is_shown());
}

#[test]
fn toggle_returns_after_hud_lease_restored() {
    let mut panel = Fixture::shown();

    panel.signal(ManagerSignal::HudLeaseLost);
    assert!(!panel.is_shown());
    assert!(panel.wants_hud_lease());

    panel.signal(ManagerSignal::HudLeaseRestored);
    assert!(panel.is_shown());
}

#[test]
fn hold_hides_on_hud_lease_lost() {
    let mut panel = Fixture::panel_with(InfoConfig { mode: InfoMode::Hold, ..InfoConfig::default() });
    panel.signal(ManagerSignal::InfoPressed);

    panel.signal(ManagerSignal::HudLeaseLost);
    assert!(!panel.is_shown());
    assert!(!panel.wants_hud_lease());

    panel.signal(ManagerSignal::HudLeaseRestored);
    assert!(!panel.is_shown());
}

#[test]
fn disabled_hides_the_panel() {
    let mut panel = Fixture::shown();

    panel.signal(ManagerSignal::Disabled);

    assert!(!panel.is_shown());
    assert!(!panel.wants_hud_lease());
}

#[test]
fn world_leave_clears_lines_and_resets_motion() {
    let mut panel = Fixture::shown();
    panel.motion(Fixture::vec3(0.0, 64.0, 0.0), 0, 0.0);
    panel.motion(Fixture::vec3(0.2, 64.0, 0.0), 0, 0.05);
    panel.refresh(&Fixture::sample_at(0.1), Fixture::names(), &[]);
    assert!(Fixture::line(&panel, "Speed:").is_some());

    panel.leave_world();
    assert_eq!(panel.lines().len(), 0);

    panel.signal(ManagerSignal::InfoPressed);
    panel.refresh(&Fixture::sample_at(0.2), Fixture::names(), &[]);
    assert!(Fixture::line(&panel, "Speed:").is_none());
    assert!(Fixture::line(&panel, "XYZ:").is_some());
}

#[test]
fn refresh_due_rules() {
    let mut panel = Fixture::panel();
    assert!(!panel.refresh_due(0.0));
    assert!(!panel.entity_scan_due(0.0));

    panel.signal(ManagerSignal::InfoPressed);
    assert!(panel.refresh_due(0.0));
    assert!(panel.entity_scan_due(0.0));

    panel.refresh(&Fixture::sample_at(10.0), Fixture::names(), &[]);
    assert!(!panel.refresh_due(10.2));
    assert!(panel.refresh_due(10.25));

    panel.submit_entities(&[], 10.0);
    assert!(!panel.entity_scan_due(10.4));
    assert!(panel.entity_scan_due(10.5));

    panel.leave_world();
    panel.signal(ManagerSignal::InfoPressed);
    assert!(panel.refresh_due(10.3));
    assert!(panel.entity_scan_due(10.3));
}

#[test]
fn due_comes_back_at_once_when_the_lease_returns() {
    let mut panel = Fixture::shown();
    panel.refresh(&Fixture::sample_at(10.0), Fixture::names(), &[]);
    panel.signal(ManagerSignal::HudLeaseLost);
    assert!(!panel.refresh_due(10.3));

    panel.signal(ManagerSignal::HudLeaseRestored);
    assert!(panel.refresh_due(10.01));
}

#[test]
fn feet_light_from_matching_surface() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);
    sample.position.y = 64.2;
    sample.sky_darken = 4;

    panel.refresh(&sample, Fixture::names(), &[Fixture::surface(62, 1, 0), Fixture::surface(63, 3, 15)]);

    assert_eq!(Fixture::line(&panel, "Light:"), Some("Light: 11 (block 3 sky 15)"));
}

#[test]
fn feet_light_missing_hides_line() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);
    sample.position.y = 64.2;

    panel.refresh(&sample, Fixture::names(), &[Fixture::surface(62, 1, 0), Fixture::surface(60, 3, 15)]);

    assert!(Fixture::line(&panel, "Light:").is_none());
}

#[test]
fn feet_light_below_zero_uses_floor() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);
    sample.position.y = -0.5;

    panel.refresh(&sample, Fixture::names(), &[Fixture::surface(-1, 9, 0), Fixture::surface(-2, 4, 0)]);

    assert_eq!(Fixture::line(&panel, "Light:"), Some("Light: 4 (block 4 sky 0)"));
}

#[test]
fn light_line_uses_band_color() {
    let mut panel = Fixture::shown();
    let red = LabelConfig::default().red;

    panel.refresh(&Fixture::sample_at(0.0), Fixture::names(), &[Fixture::surface(63, 0, 0)]);

    assert_eq!(Fixture::line(&panel, "Light:"), Some("Light: 0 (block 0 sky 0)"));
    assert_eq!(Fixture::color_of(&panel, "Light:"), Some([red[0], red[1], red[2]]));
}

#[test]
fn position_lines_use_floor_and_chunk_offsets() {
    let mut panel = Fixture::shown();

    panel.refresh(&Fixture::sample_at(0.0), Fixture::names(), &[]);

    assert_eq!(Fixture::line(&panel, "XYZ:"), Some("XYZ: 12.345 / 64.000 / -3.250"));
    assert_eq!(Fixture::line(&panel, "Standing on:"), Some("Standing on: 12 63 -4  Chunk: 0 -1 in 12 12"));
    assert_eq!(Fixture::line(&panel, "Facing:"), Some("Facing: north (yaw 180.0 pitch 12.5)"));
    assert_eq!(Fixture::line(&panel, "Dimension:"), Some("Dimension: overworld  Biome: deep_dark"));
}

#[test]
fn facing_follows_bedrock_yaw() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);
    let cases = [(0.0, "south"), (90.0, "west"), (-90.0, "east"), (-179.0, "north"), (44.0, "south")];

    for (yaw, direction) in cases {
        sample.yaw = yaw;
        panel.refresh(&sample, Fixture::names(), &[]);
        let text = Fixture::line(&panel, "Facing:").unwrap();
        assert!(text.starts_with(&format!("Facing: {direction} ")), "yaw {yaw}: {text}");
    }
}

#[test]
fn slime_line_overworld_only() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);

    panel.refresh(&sample, Fixture::names(), &[]);
    assert!(Fixture::line(&panel, "Slime chunk:").is_some());

    sample.dimension = 1;
    panel.refresh(&sample, Fixture::names(), &[]);
    assert!(Fixture::line(&panel, "Slime chunk:").is_none());
    assert_eq!(Fixture::line(&panel, "Dimension:"), Some("Dimension: nether  Biome: deep_dark"));
}

#[test]
fn slime_chunk_is_green() {
    let mut panel = Fixture::shown();
    let green = LabelConfig::default().green;
    let mut sample = Fixture::sample_at(0.0);
    let slime = (-64..64)
        .flat_map(|x| (-64..64).map(move |z| (x, z)))
        .find(|&(x, z)| levels_core::services::SlimeChunks::is_slime(x, z))
        .unwrap();
    sample.position.x = slime.0 as f32 * 16.0 + 1.0;
    sample.position.z = slime.1 as f32 * 16.0 + 1.0;

    panel.refresh(&sample, Fixture::names(), &[]);

    assert_eq!(Fixture::line(&panel, "Slime chunk:"), Some("Slime chunk: yes"));
    assert_eq!(Fixture::color_of(&panel, "Slime chunk:"), Some([green[0], green[1], green[2]]));
}

#[test]
fn missing_biome_hides_biome_only() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);
    sample.biome = None;

    panel.refresh(&sample, InfoNames { biome: None, target_block: None }, &[]);

    assert_eq!(Fixture::line(&panel, "Dimension:"), Some("Dimension: overworld"));
}

#[test]
fn block_target_line() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);
    sample.target = InfoTarget::Block { pos: BlockPos { x: 10, y: 63, z: -4 }, face: 1 };

    panel.refresh(&sample, Fixture::names(), &[]);

    assert_eq!(Fixture::line(&panel, "Target:"), Some("Target: minecraft:stone 10 63 -4 face up"));
}

#[test]
fn entity_target_lines() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);
    sample.position = Fixture::vec3(0.0, 64.0, 0.0);
    panel.submit_entities(
        &[
            Fixture::actor(Fixture::vec3(2.7, 64.0, -0.3), Fixture::vec3(3.3, 66.0, 0.3), ZOMBIE),
            Fixture::actor(Fixture::vec3(-0.3, 64.0, 4.7), Fixture::vec3(0.3, 66.0, 5.3), UNNAMED_HOSTILE),
        ],
        0.0,
    );

    sample.target = InfoTarget::Entity { point: Fixture::vec3(3.2, 64.0, 0.0) };
    panel.refresh(&sample, Fixture::names(), &[]);
    assert_eq!(Fixture::line(&panel, "Target:"), Some("Target: zombie 3.2 m"));

    sample.target = InfoTarget::Entity { point: Fixture::vec3(0.0, 64.0, 5.0) };
    panel.refresh(&sample, Fixture::names(), &[]);
    assert_eq!(Fixture::line(&panel, "Target:"), Some("Target: #60 5.0 m"));

    sample.target = InfoTarget::Entity { point: Fixture::vec3(-9.0, 64.0, 0.0) };
    panel.refresh(&sample, Fixture::names(), &[]);
    assert_eq!(Fixture::line(&panel, "Target:"), Some("Target: entity 9.0 m"));
}

#[test]
fn entity_lines_count_and_name_hostiles() {
    let mut panel = Fixture::shown();
    let zero = Fixture::vec3(0.0, 0.0, 0.0);
    panel.submit_entities(
        &[
            Fixture::actor(zero, zero, ZOMBIE),
            Fixture::actor(zero, zero, ZOMBIE),
            Fixture::actor(zero, zero, UNNAMED_HOSTILE),
            Fixture::actor(zero, zero, 0x0B | 0x100),
            Fixture::actor(zero, zero, 0x40),
        ],
        0.0,
    );

    panel.refresh(&Fixture::sample_at(0.0), Fixture::names(), &[]);

    assert_eq!(Fixture::line(&panel, "Entities:"), Some("Entities: 5  players 0  hostile 3  passive 1  other 1"));
    assert_eq!(Fixture::line(&panel, "Hostile:"), Some("Hostile: zombie 2  #60 1"));
}

#[test]
fn census_unavailable_hides_the_entity_lines_until_the_next_scan() {
    let mut panel = Fixture::shown();
    let zero = Fixture::vec3(0.0, 0.0, 0.0);
    panel.submit_entities(&[Fixture::actor(zero, zero, ZOMBIE)], 0.0);
    panel.refresh(&Fixture::sample_at(0.0), Fixture::names(), &[]);
    assert!(Fixture::line(&panel, "Entities:").is_some());

    panel.census_unavailable();
    panel.refresh(&Fixture::sample_at(1.0), Fixture::names(), &[]);

    assert_eq!(Fixture::line(&panel, "Entities:"), None);
    assert_eq!(Fixture::line(&panel, "Hostile:"), None);
    assert!(panel.entity_counts().is_none());
    panel.submit_entities(&[Fixture::actor(zero, zero, ZOMBIE)], 2.0);
    panel.refresh(&Fixture::sample_at(2.0), Fixture::names(), &[]);
    assert_eq!(Fixture::line(&panel, "Entities:"), Some("Entities: 1  players 0  hostile 1  passive 0  other 0"));
}

#[test]
fn performance_and_amml_lines() {
    let mut panel = Fixture::shown();
    panel.frames(0, 0.0);
    panel.frame_cost(200.0);
    panel.frame_cost(220.0);
    panel.frames(144, 1.0);
    panel.levels_memory(12_900_000, 32 * 1024 * 1024);
    panel.set_world(levels_core::structs::info::InfoWorldFacts {
        level_name: Some("My World"),
        seed: Some(42),
        connection: levels_core::structs::info::InfoConnection::Local,
    });

    panel.refresh(&Fixture::sample_at(1.0), Fixture::names(), &[]);

    assert_eq!(Fixture::line(&panel, "FPS:"), Some("FPS: 144  Frame: 6.9 ms"));
    assert_eq!(Fixture::line(&panel, "World:"), Some("World: My World  Seed: 42  Local"));
    assert_eq!(Fixture::line(&panel, "Levels:"), Some("Levels: 0.21 ms  12.3 / 32 MB"));
}

#[test]
fn standing_on_is_the_block_under_the_feet() {
    let mut panel = Fixture::shown();
    let mut sample = Fixture::sample_at(0.0);

    sample.position = Fixture::vec3(12.345, 63.5, -3.25);
    panel.refresh(&sample, Fixture::names(), &[]);
    assert_eq!(Fixture::line(&panel, "Standing on:"), Some("Standing on: 12 63 -4  Chunk: 0 -1 in 12 12"));

    sample.position = Fixture::vec3(12.345, -64.0, -3.25);
    panel.refresh(&sample, Fixture::names(), &[]);
    assert_eq!(Fixture::line(&panel, "Standing on:"), Some("Standing on: 12 -65 -4  Chunk: 0 -1 in 12 12"));
}

#[test]
fn group_off_hides_its_lines() {
    let mut panel = Fixture::panel_with(InfoConfig { show_position: false, ..InfoConfig::default() });
    panel.signal(ManagerSignal::InfoPressed);

    panel.refresh(&Fixture::sample_at(0.0), Fixture::names(), &[]);

    assert!(Fixture::line(&panel, "XYZ:").is_none());
    assert!(Fixture::line(&panel, "Standing on:").is_none());
    assert!(Fixture::line(&panel, "Facing:").is_none());
    assert!(Fixture::line(&panel, "Dimension:").is_some());
}

#[test]
fn feet_scan_range_is_the_block_under_the_feet() {
    let panel = Fixture::shown();

    let request = panel.feet_scan_range(Fixture::vec3(12.3, 64.0, -3.2));
    let below_zero = panel.feet_scan_range(Fixture::vec3(0.5, -0.25, 0.5));

    assert_eq!(request, ColumnRequest { column: ColumnPos { x: 12, z: -4 }, y_min: 63, y_max: 63 });
    assert_eq!((below_zero.y_min, below_zero.y_max), (-2, -2));
}

#[test]
fn entity_scan_box_is_the_radius_around_the_position() {
    let panel = Fixture::shown();
    let wide = Fixture::panel_with(InfoConfig { entity_radius: 500.0, ..InfoConfig::default() });

    let (min, max) = panel.entity_scan_box(Fixture::vec3(10.0, 64.0, -20.0));
    let (wide_min, wide_max) = wide.entity_scan_box(Fixture::vec3(0.0, 0.0, 0.0));

    assert_eq!((min, max), (Fixture::vec3(-22.0, 32.0, -52.0), Fixture::vec3(42.0, 96.0, 12.0)));
    assert_eq!((wide_min, wide_max), (Fixture::vec3(-64.0, -64.0, -64.0), Fixture::vec3(64.0, 64.0, 64.0)));
}

#[test]
fn feet_block_is_the_block_the_feet_are_in() {
    let panel = Fixture::shown();

    assert_eq!(panel.feet_block(Fixture::vec3(12.3, 64.0, -3.2)), BlockPos { x: 12, y: 64, z: -4 });
    assert_eq!(panel.feet_block(Fixture::vec3(0.5, -0.25, 0.5)), BlockPos { x: 0, y: -1, z: 0 });
}

#[test]
fn place_change_drops_the_old_lines_and_makes_a_refresh_and_scan_due() {
    let mut panel = Fixture::shown();
    let zero = Fixture::vec3(0.0, 0.0, 0.0);
    panel.submit_entities(&[Fixture::actor(zero, zero, ZOMBIE)], 0.0);
    panel.refresh(&Fixture::sample_at(0.0), Fixture::names(), &[]);
    assert!(Fixture::line(&panel, "Entities:").is_some());
    assert!(!panel.refresh_due(0.01));
    assert!(!panel.entity_scan_due(0.01));

    panel.place_changed();

    assert_eq!(panel.lines().len(), 0);
    assert!(panel.entity_counts().is_none());
    assert!(panel.refresh_due(0.01));
    assert!(panel.entity_scan_due(0.01));
    assert!(panel.is_shown());
}

#[test]
fn background_color_is_black() {
    assert_eq!(Fixture::panel().background_color(), [0.0, 0.0, 0.0]);
}
