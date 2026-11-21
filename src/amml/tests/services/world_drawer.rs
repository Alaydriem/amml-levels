use std::sync::{Arc, Mutex};

use alaydriem_levels::services::{LevelsLog, MeshSlot, WorldDrawer};
use alaydriem_levels::types::WorldDrawStats;
use amm_client::test_support::FakeTable;
use amml_api::Area;
use amml_api::testing::{FakeCanvas, FakeGame};
use levels_core::structs::draw::ColorQuad;
use levels_core::structs::marker::{MarkerInstance, WorldDrawPlan};
use levels_core::structs::world::BlockPos;

use crate::support::probe::RenderProbe;

const QUAD: ColorQuad = ColorQuad { corners: [[0.0; 3]; 4], rgba: [255, 0, 0, 255] };

// a drawer on a fake game with the canvas area offered, and the canvas the next frames use
struct Rig {
    game: FakeGame,
    drawer: Arc<Mutex<WorldDrawer>>,
    canvas: FakeCanvas,
}

impl Rig {
    fn new() -> Self {
        let game = FakeTable::game();
        game.offer::<amml_api::amml_abi::WorldRendered>();
        game.offer_area(Area::RENDER_WORLD_CANVAS.at(2));
        let drawer = Arc::new(Mutex::new(WorldDrawer::new(LevelsLog::new(game.ctx().log()))));
        Self { game, drawer, canvas: FakeCanvas::default() }
    }

    fn camera_at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.canvas.camera_position = Some(amml_api::Vec3 { x, y, z });
        self
    }

    // draws `plan` in one frame; the calls of the frame are in `self.canvas.calls` (cleared first)
    fn render(&self, plan: Plan) -> WorldDrawStats {
        self.canvas.calls.borrow_mut().clear();
        self.canvas.frees.borrow_mut().clear();
        let drawer = Arc::clone(&self.drawer);
        let stats = Arc::new(Mutex::new(WorldDrawStats::default()));
        let out = Arc::clone(&stats);
        RenderProbe::frame(&self.game, &self.canvas, move |canvas| {
            *out.lock().unwrap() = drawer.lock().unwrap().render(canvas, plan.view());
        });
        Arc::try_unwrap(stats).unwrap().into_inner().unwrap()
    }

    fn calls(&self) -> Vec<String> {
        self.canvas.calls.borrow().clone()
    }
}

// owns what a `WorldDrawPlan` borrows
#[derive(Clone, Default)]
struct Plan {
    none: bool,
    label_version: u64,
    label_quads: Vec<ColorQuad>,
    shell_version: u64,
    shell_quads: Vec<ColorQuad>,
    instances: Vec<MarkerInstance>,
}

impl Plan {
    fn labels(version: u64, quads: usize) -> Self {
        Self { label_version: version, label_quads: vec![QUAD; quads], ..Self::default() }
    }

    fn view(&self) -> Option<WorldDrawPlan<'_>> {
        (!self.none).then_some(WorldDrawPlan {
            label_origin: BlockPos { x: 10, y: 63, z: -20 },
            label_version: self.label_version,
            label_quads: &self.label_quads,
            border_origin: BlockPos::default(),
            border_version: 0,
            border_quads: &[],
            shell_version: self.shell_version,
            shell_quads: &self.shell_quads,
            instances: &self.instances,
        })
    }
}

fn builds(calls: &[String]) -> Vec<&String> {
    calls.iter().filter(|call| call.starts_with("build")).collect()
}

fn draws(calls: &[String]) -> Vec<&String> {
    calls.iter().filter(|call| call.starts_with("draw")).collect()
}

#[test]
fn a_list_over_the_loader_limit_is_split_into_meshes_of_at_most_the_limit() {
    let rig = Rig::new();

    let stats = rig.render(Plan::labels(1, MeshSlot::MAX_QUADS + 1809));

    let calls = rig.calls();
    assert_eq!(
        builds(&calls),
        [
            &format!("build quads={} style=alpha/1/0/2/full/white", MeshSlot::MAX_QUADS),
            "build quads=1809 style=alpha/1/0/2/full/white"
        ]
    );
    assert_eq!(draws(&calls).len(), 2);
    assert_eq!((stats.meshes_built, stats.meshes_drawn), (2, 2));
}

#[test]
fn a_plan_version_is_built_once_and_a_new_version_frees_the_old_meshes() {
    let rig = Rig::new();
    rig.render(Plan::labels(1, 4));

    rig.render(Plan::labels(1, 4));
    assert!(builds(&rig.calls()).is_empty());
    assert_eq!(draws(&rig.calls()).len(), 1);

    rig.render(Plan::labels(2, 8));
    assert_eq!(builds(&rig.calls()).len(), 1);
    assert_eq!(rig.canvas.frees.borrow().len(), 1);
    assert_eq!(rig.game.live_meshes(), 1);
}

#[test]
fn a_label_draw_lands_at_the_label_origin_unturned() {
    let rig = Rig::new();

    rig.render(Plan::labels(1, 4));

    let calls = rig.calls();
    assert!(draws(&calls)[0].ends_with("10 63 -20 yaw=0"), "{calls:?}");
}

#[test]
fn no_plan_frees_every_mesh() {
    let rig = Rig::new();
    rig.render(Plan::labels(1, 4));

    let stats = rig.render(Plan { none: true, ..Plan::default() });

    assert_eq!(rig.canvas.frees.borrow().len(), 1);
    assert_eq!(rig.game.live_meshes(), 0);
    assert_eq!(stats, WorldDrawStats::default());
}

#[test]
fn an_empty_quad_list_builds_and_draws_nothing() {
    let rig = Rig::new();

    let stats = rig.render(Plan::labels(0, 0));

    assert!(rig.calls().is_empty());
    assert_eq!(stats.meshes_drawn, 0);
}

#[test]
fn shells_are_drawn_farthest_first_with_the_spin_negated() {
    let rig = Rig::new().camera_at(0.0, 64.0, 0.0);
    let plan = Plan {
        shell_version: 1,
        shell_quads: vec![QUAD; 6],
        instances: vec![
            MarkerInstance { center: [4.5, 64.5, 0.5], spin_degrees: 30.0 },
            MarkerInstance { center: [40.5, 64.5, 0.5], spin_degrees: 30.0 },
            MarkerInstance { center: [20.5, 64.5, 0.5], spin_degrees: -10.0 },
        ],
        ..Plan::default()
    };

    rig.render(plan);

    let calls = rig.calls();
    let order: Vec<&str> = draws(&calls).iter().map(|call| call.split(' ').nth(2).unwrap()).collect();
    assert_eq!(order, ["40.5", "20.5", "4.5"]);
    let yaws: Vec<&str> = draws(&calls).iter().map(|call| call.rsplit("yaw=").next().unwrap()).collect();
    assert_eq!(yaws, ["-30", "10", "-30"]);
}

#[test]
fn a_shell_far_from_the_origin_keeps_its_sub_block_position() {
    let rig = Rig::new();
    let plan = Plan {
        shell_version: 1,
        shell_quads: vec![QUAD; 6],
        instances: vec![MarkerInstance { center: [-1000000.5, 64.25, 999999.75], spin_degrees: 0.0 }],
        ..Plan::default()
    };

    rig.render(plan);

    let calls = rig.calls();
    let draw = draws(&calls)[0];
    assert!(draw.contains("-1000000.5 64.25 999999.75"), "{draw}");
}

#[test]
fn a_graphics_mode_switch_rebuilds_and_draws_in_the_same_frame() {
    let rig = Rig::new();
    rig.render(Plan::labels(1, 4));

    rig.game.switch_graphics();
    let stats = rig.render(Plan::labels(1, 4));

    let calls = rig.calls();
    assert_eq!(builds(&calls).len(), 1);
    assert_eq!(draws(&calls).len(), 1);
    assert_eq!((stats.meshes_built, stats.meshes_drawn), (1, 1));
    assert_eq!(rig.game.live_meshes(), 1);
}

#[test]
fn after_a_world_leave_the_drawer_builds_again_without_freeing_stale_ids() {
    let rig = Rig::new();
    rig.render(Plan::labels(1, 4));
    rig.game.leave_mesh_world();
    rig.drawer.lock().unwrap().leave();

    rig.render(Plan::labels(1, 4));

    assert_eq!(builds(&rig.calls()).len(), 1);
    assert!(rig.canvas.frees.borrow().is_empty());
}

#[test]
fn the_buffers_are_sized_once() {
    let rig = Rig::new();
    let before = rig.drawer.lock().unwrap().resident_bytes();

    rig.render(Plan::labels(1, MeshSlot::MAX_QUADS));

    assert_eq!(rig.drawer.lock().unwrap().resident_bytes(), before);
}
