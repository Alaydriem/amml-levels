use alaydriem_levels::services::{FakeMeshFactory, MeshFactory, MeshSlot, QuadWriter, WorldDrawer};
use alaydriem_levels::types::{MeshCalls, WorldFrame, WorldSlot};
use levels_core::config::BordersConfig;
use levels_core::services::ChunkBorderPlanner;
use levels_core::structs::draw::ColorQuad;
use levels_core::structs::marker::{MarkerInstance, WorldDrawPlan};
use levels_core::structs::world::BlockPos;

use crate::support::{FakeMeshEntries, LogCapture};

const FAILED_ONCE: &str = "a world mesh build failed; it is tried again next frame";
const GAVE_UP: &str = "a world mesh build failed again; the slot stays empty until its quads change";
const STALLED: &str = "a world mesh build keeps waiting on the game; it is still tried each frame";

const QUAD: ColorQuad = ColorQuad {
    corners: [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
    rgba: [0xFF, 0x20, 0x20, 0x80],
};
const QUADS: [ColorQuad; 2] = [QUAD, QUAD];
const CAMERA: [f32; 3] = [30_000.25, 64.0, 30_000.75];

fn drawer() -> WorldDrawer {
    WorldDrawer::new(MeshFactory::Fake(FakeMeshFactory::new()))
}

fn drawer_with(fake: FakeMeshFactory) -> WorldDrawer {
    WorldDrawer::new(MeshFactory::Fake(fake))
}

fn frame(calls: &MeshCalls) -> WorldFrame<'_> {
    WorldFrame {
        calls,
        screen: std::ptr::null_mut(),
        client: std::ptr::null_mut(),
        material: std::ptr::null(),
        camera: CAMERA,
    }
}

fn plan(border_version: u64, shell_version: u64, instances: &[MarkerInstance]) -> WorldDrawPlan<'_> {
    WorldDrawPlan {
        label_origin: BlockPos { x: 30_000, y: 63, z: 29_999 },
        label_version: 1,
        label_quads: &[],
        border_origin: BlockPos { x: 29_984, y: -64, z: 29_984 },
        border_version,
        border_quads: &QUADS,
        shell_version,
        shell_quads: &QUADS,
        instances,
    }
}

// a plan with only border quads, so each frame tries one build at most
fn border_plan(border_version: u64, border_quads: &[ColorQuad]) -> WorldDrawPlan<'_> {
    WorldDrawPlan { border_quads, shell_quads: &[], ..plan(border_version, 1, &[]) }
}

fn border_draws(drawer: &WorldDrawer) -> usize {
    slot_draws(drawer, WorldSlot::Border)
}

fn slot_draws(drawer: &WorldDrawer, wanted: WorldSlot) -> usize {
    fake(drawer).draws().iter().filter(|(slot, _)| *slot == wanted).count()
}

fn fake(drawer: &WorldDrawer) -> &FakeMeshFactory {
    match drawer.factory() {
        MeshFactory::Fake(fake) => fake,
        MeshFactory::Game(_) => panic!("not the fake factory"),
    }
}

fn instance(center: [f32; 3], spin_degrees: f32) -> MarkerInstance {
    MarkerInstance { center, spin_degrees }
}

#[test]
#[allow(unsafe_code)]
fn no_plan_builds_nothing() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();

    // SAFETY: the fake factory touches no game object
    let stats = unsafe { drawer.render(&frame(&calls), None) };

    assert_eq!(stats.meshes_built, 0);
    assert_eq!(fake(&drawer).builds(), 0);
    assert!(fake(&drawer).draws().is_empty());
    assert_eq!(drawer.resident_bytes(), 0);
}

#[test]
#[allow(unsafe_code)]
fn rebuild_only_when_the_version_changes() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();

    // SAFETY: the fake factory touches no game object
    let first = unsafe { drawer.render(&frame(&calls), Some(plan(1, 1, &[]))) };
    // SAFETY: as above
    let same = unsafe { drawer.render(&frame(&calls), Some(plan(1, 1, &[]))) };
    // SAFETY: as above
    let none = unsafe { drawer.render(&frame(&calls), None) };
    // SAFETY: as above
    let border = unsafe { drawer.render(&frame(&calls), Some(plan(2, 1, &[]))) };
    // SAFETY: as above
    let shells = unsafe { drawer.render(&frame(&calls), Some(plan(2, 2, &[]))) };

    assert_eq!(first.meshes_built, 2);
    assert_eq!(same.meshes_built, 0);
    assert_eq!(none.meshes_built, 0);
    assert_eq!(border.meshes_built, 1);
    assert_eq!(shells.meshes_built, 1);
    assert_eq!(fake(&drawer).builds(), 4);
    assert!(drawer.resident_bytes() > 0, "a None plan keeps the meshes");
}

#[test]
#[allow(unsafe_code)]
fn leave_drops_every_mesh() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();
    // SAFETY: the fake factory touches no game object
    unsafe { drawer.render(&frame(&calls), Some(plan(1, 1, &[]))) };
    assert!(drawer.resident_bytes() > 0);

    drawer.leave();

    assert_eq!(drawer.resident_bytes(), 0);
    // SAFETY: as above
    let again = unsafe { drawer.render(&frame(&calls), Some(plan(1, 1, &[]))) };
    assert_eq!(again.meshes_built, 2, "the same versions build again after a leave");
}

#[test]
#[allow(unsafe_code)]
fn draws_the_border_then_one_shell_per_marker_farthest_first() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();
    let near = [30_004.5, 64.0, 30_000.75];
    let far = [30_020.5, 64.0, 30_000.75];
    let instances = [instance(near, 0.0), instance(far, 0.0)];

    // SAFETY: the fake factory touches no game object
    unsafe { drawer.render(&frame(&calls), Some(plan(1, 1, &instances))) };

    let slots: Vec<WorldSlot> = fake(&drawer).draws().iter().map(|(slot, _)| *slot).collect();
    let xs: Vec<f32> = fake(&drawer).draws().iter().map(|(_, offset)| offset[0]).collect();
    assert_eq!(slots, [WorldSlot::Border, WorldSlot::Shell, WorldSlot::Shell]);
    assert_eq!(xs, [-16.25, 20.25, 4.25]);
}

#[test]
#[allow(unsafe_code)]
fn offsets_are_exact_far_from_the_world_origin() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();

    // SAFETY: the fake factory touches no game object
    unsafe { drawer.render(&frame(&calls), Some(plan(1, 1, &[]))) };

    let (slot, offset) = fake(&drawer).draws()[0];
    assert_eq!(slot, WorldSlot::Border);
    assert_eq!(offset, [-16.25, -128.0, -16.75]);
}

#[test]
#[allow(unsafe_code)]
fn a_grid_plan_fits_one_border_mesh() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();
    let mut planner = ChunkBorderPlanner::new(BordersConfig::default());
    planner.cycle();
    planner.cycle();
    planner.update(BlockPos { x: 0, y: 70, z: 0 }, 0);
    assert!(planner.quads().len() <= MeshSlot::MAX_QUADS);
    assert!(MeshSlot::MAX_QUADS * QuadWriter::MAX_VERTICES_PER_QUAD <= usize::from(u16::MAX));
    assert_eq!(MeshSlot::meshes_for(MeshSlot::MAX_QUADS), 1);
    assert_eq!(MeshSlot::meshes_for(MeshSlot::MAX_QUADS + 1), 2);

    // SAFETY: the fake factory touches no game object
    let stats = unsafe { drawer.render(&frame(&calls), Some(border_plan(planner.version(), planner.quads()))) };

    assert_eq!(WorldDrawer::BORDER_MESHES, 1);
    assert_eq!(stats.meshes_built, 1);
    assert_eq!(border_draws(&drawer), 1);
    let vertices = QuadWriter::VERTICES_PER_QUAD as u64 * planner.quads().len() as u64;
    assert!(drawer.resident_bytes() >= vertices * 24, "the mesh holds every vertex");
}

#[test]
#[allow(unsafe_code)]
fn a_failed_build_is_tried_again_next_frame() {
    LogCapture::install();
    let calls = FakeMeshEntries::calls();
    let mut factory = FakeMeshFactory::new();
    factory.fail_next(1);
    let mut drawer = drawer_with(factory);

    // SAFETY: the fake factory touches no game object
    let first = unsafe { drawer.render(&frame(&calls), Some(border_plan(1, &QUADS))) };
    assert_eq!(first.meshes_built, 0);
    assert_eq!(border_draws(&drawer), 0);
    // SAFETY: as above
    let second = unsafe { drawer.render(&frame(&calls), Some(border_plan(1, &QUADS))) };

    assert_eq!(second.meshes_built, 1, "the same version is built again");
    assert_eq!(border_draws(&drawer), 1);
    assert_eq!(fake(&drawer).builds(), 2);
    assert_eq!(LogCapture::count(FAILED_ONCE), 1);
    assert_eq!(LogCapture::count(GAVE_UP), 0);
}

#[test]
#[allow(unsafe_code)]
fn failed_builds_stop_after_the_limit_and_log_once() {
    LogCapture::install();
    let calls = FakeMeshEntries::calls();
    let mut factory = FakeMeshFactory::new();
    factory.fail_next(u32::MAX);
    let mut drawer = drawer_with(factory);

    for _ in 0..10 {
        // SAFETY: the fake factory touches no game object
        unsafe { drawer.render(&frame(&calls), Some(border_plan(1, &QUADS))) };
    }

    assert_eq!(fake(&drawer).builds(), MeshSlot::MAX_FAILURES);
    assert_eq!(border_draws(&drawer), 0);
    assert_eq!(drawer.resident_bytes(), 0);
    assert_eq!(LogCapture::count(FAILED_ONCE), 1);
    assert_eq!(LogCapture::count(GAVE_UP), 1);
    // SAFETY: as above
    unsafe { drawer.render(&frame(&calls), Some(border_plan(2, &QUADS))) };
    assert_eq!(fake(&drawer).builds(), MeshSlot::MAX_FAILURES + 1, "a new version is tried again");
}

#[test]
#[allow(unsafe_code)]
fn a_busy_build_is_tried_each_frame_and_the_stall_is_logged_once() {
    LogCapture::install();
    let calls = FakeMeshEntries::calls();
    let frames = MeshSlot::STALL_FRAMES + 10;
    let mut factory = FakeMeshFactory::new();
    factory.busy_next(frames);
    let mut drawer = drawer_with(factory);

    for _ in 0..frames {
        // SAFETY: the fake factory touches no game object
        unsafe { drawer.render(&frame(&calls), Some(border_plan(1, &QUADS))) };
    }
    assert_eq!(border_draws(&drawer), 0);
    // SAFETY: as above
    let built = unsafe { drawer.render(&frame(&calls), Some(border_plan(1, &QUADS))) };

    assert_eq!(built.meshes_built, 1);
    assert_eq!(border_draws(&drawer), 1);
    assert_eq!(fake(&drawer).builds(), frames + 1);
    assert_eq!(LogCapture::count(STALLED), 1);
    assert_eq!(LogCapture::count(FAILED_ONCE), 0);
}

#[test]
#[allow(unsafe_code)]
fn label_slot_builds_and_draws_with_the_plan() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();
    let labeled = WorldDrawPlan { label_quads: &QUADS, ..border_plan(1, &[]) };

    // SAFETY: the fake factory touches no game object
    let first =
        unsafe { drawer.render(&frame(&calls), Some(WorldDrawPlan { label_quads: &QUADS, ..border_plan(1, &[]) })) };
    // SAFETY: as above
    let same = unsafe { drawer.render(&frame(&calls), Some(labeled)) };

    assert_eq!(first.meshes_built, 1);
    assert_eq!(same.meshes_built, 0, "the same label version is not built again");
    let draws = fake(&drawer).draws();
    assert_eq!(draws[0].0, WorldSlot::Label, "the labels are drawn first");
    assert_eq!(draws[0].1, [-0.25, -1.0, -1.75]);
    assert_eq!(slot_draws(&drawer, WorldSlot::Label), 1);
    assert_eq!(WorldDrawer::LABEL_MESHES, MeshSlot::meshes_for(16 * levels_core::config::MAX_LABELS as usize));
}

#[test]
#[allow(unsafe_code)]
fn changing_the_sides_rebuilds_every_slot_and_the_same_sides_rebuild_nothing() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();
    let plan = || WorldDrawPlan { label_quads: &QUADS, ..border_plan(1, &QUADS) };
    // SAFETY: the fake factory touches no game object
    let first = unsafe { drawer.render(&frame(&calls), Some(plan())) };
    assert_eq!(first.meshes_built, 2);

    drawer.set_two_sided(false);
    // SAFETY: as above
    let same = unsafe { drawer.render(&frame(&calls), Some(plan())) };
    assert_eq!(same.meshes_built, 0);

    drawer.set_two_sided(true);
    // SAFETY: as above
    let changed = unsafe { drawer.render(&frame(&calls), Some(plan())) };
    assert_eq!(changed.meshes_built, 2);
}

#[test]
#[allow(unsafe_code)]
fn label_quads_past_one_mesh_split_into_two() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();
    let quads = vec![QUAD; MeshSlot::MAX_QUADS + 1];
    let split = WorldDrawPlan { label_quads: &quads, ..border_plan(1, &[]) };

    // SAFETY: the fake factory touches no game object
    let stats = unsafe { drawer.render(&frame(&calls), Some(split)) };

    assert_eq!(stats.meshes_built, 2);
    assert_eq!(slot_draws(&drawer, WorldSlot::Label), 2);
}

#[test]
#[allow(unsafe_code)]
fn a_hidden_hud_plan_empties_the_label_slot_and_the_next_version_rebuilds_it() {
    let calls = FakeMeshEntries::calls();
    let mut drawer = drawer();
    let shown = || WorldDrawPlan { label_quads: &QUADS, ..border_plan(1, &[]) };
    let hidden = || WorldDrawPlan { label_version: 0, label_quads: &[], ..border_plan(1, &[]) };
    // SAFETY: the fake factory touches no game object
    unsafe { drawer.render(&frame(&calls), Some(shown())) };
    assert!(drawer.resident_bytes() > 0);

    // SAFETY: as above
    let gone = unsafe { drawer.render(&frame(&calls), Some(hidden())) };

    assert_eq!(gone.meshes_built, 0);
    assert_eq!(slot_draws(&drawer, WorldSlot::Label), 0);
    assert_eq!(drawer.resident_bytes(), 0);
    // SAFETY: as above
    let back = unsafe { drawer.render(&frame(&calls), Some(WorldDrawPlan { label_version: 2, ..shown() })) };
    assert_eq!(back.meshes_built, 1);
    assert_eq!(slot_draws(&drawer, WorldSlot::Label), 1);
}
