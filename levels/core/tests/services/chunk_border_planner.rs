use levels_core::config::BordersConfig;
use levels_core::services::ChunkBorderPlanner;
use levels_core::structs::draw::ColorQuad;
use levels_core::structs::marker::BorderMode;
use levels_core::structs::world::BlockPos;

fn at(x: i32, z: i32) -> BlockPos {
    BlockPos { x, y: 70, z }
}

fn planner(mode_steps: usize) -> ChunkBorderPlanner {
    let mut planner = ChunkBorderPlanner::new(BordersConfig::default());
    for _ in 0..mode_steps {
        planner.cycle();
    }
    planner
}

const INSET: f32 = 0.02;

fn gold_config() -> BordersConfig {
    BordersConfig { center_color: [1.0, 0.784, 0.235, 0.376], ..BordersConfig::default() }
}

fn gold_rgb() -> [u8; 3] {
    let c = gold_config().center_color;
    [c[0], c[1], c[2]].map(|v| (v * 255.0).round() as u8)
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

// the fixed coordinate of every wall quad, x walls first
fn wall_planes(planner: &ChunkBorderPlanner) -> Vec<(bool, f32)> {
    planner
        .quads()
        .iter()
        .filter_map(|q| {
            if q.corners.iter().all(|c| c[0] == q.corners[0][0]) {
                Some((true, q.corners[0][0]))
            } else if q.corners.iter().all(|c| c[2] == q.corners[0][2]) {
                Some((false, q.corners[0][2]))
            } else {
                None
            }
        })
        .collect()
}

fn is_horizontal(quad: &ColorQuad) -> bool {
    let ys = quad.corners.map(|c| c[1]);
    let span = ys.iter().cloned().fold(f32::MIN, f32::max) - ys.iter().cloned().fold(f32::MAX, f32::min);
    span < 1.0
}

#[test]
fn cycle_goes_off_single_grid_off() {
    let mut planner = planner(0);
    assert_eq!(planner.mode(), BorderMode::Off);
    planner.cycle();
    assert_eq!(planner.mode(), BorderMode::Single);
    planner.cycle();
    assert_eq!(planner.mode(), BorderMode::Grid);
    planner.cycle();
    assert_eq!(planner.mode(), BorderMode::Off);
}

#[test]
fn walls_snap_for_negative_and_edge_positions() {
    for x in [-17, -16, -1, 0, 15, 16] {
        let mut planner = planner(1);
        planner.update(at(x, 0), 0);
        assert_eq!(planner.origin().x, x.div_euclid(16) * 16 + 8);
        let x_walls: Vec<_> =
            planner.quads().iter().filter(|q| q.corners.iter().all(|c| c[0] == q.corners[0][0])).collect();
        assert!(!x_walls.is_empty());
        for quad in x_walls {
            let x = quad.corners[0][0];
            assert!(near(x, INSET - 8.0) || near(x, 8.0 - INSET));
        }
    }
}

#[test]
fn single_overworld_quad_count() {
    let mut planner = planner(1);
    planner.update(at(0, 0), 0);
    assert_eq!(planner.quads().len(), 4 * 33 + 64);
}

#[test]
fn grid_center_segments_are_gold() {
    let mut planner = ChunkBorderPlanner::new(gold_config());
    planner.cycle();
    planner.cycle();
    planner.update(at(0, 0), 0);
    let gold = gold_rgb();
    let count = planner.quads().iter().filter(|q| is_horizontal(q) && q.rgba[..3] == gold).count();
    assert_eq!(count, 4 * 33);
}

fn band_at(feet_y: i32, dimension: i32) -> ChunkBorderPlanner {
    let mut planner = planner(1);
    planner.update(BlockPos { x: 0, y: feet_y, z: 0 }, dimension);
    planner
}

// the lowest and highest y of the quads, in world blocks
fn world_y_span(planner: &ChunkBorderPlanner) -> (f32, f32) {
    let ys = planner.quads().iter().flat_map(|q| q.corners).map(|c| c[1] + planner.origin().y as f32);
    ys.fold((f32::MAX, f32::MIN), |(lo, hi), y| (lo.min(y), hi.max(y)))
}

#[test]
fn the_band_covers_the_feet_section_and_eight_blocks_each_side() {
    let planner = band_at(70, 0);
    assert_eq!(world_y_span(&planner), (56.0, 88.0));
    assert_eq!(planner.origin().y, 72);
}

#[test]
fn the_band_stays_inside_the_build_range() {
    assert_eq!(world_y_span(&band_at(-64, 0)), (-64.0, -32.0));
    assert_eq!(world_y_span(&band_at(319, 0)), (288.0, 320.0));
    assert_eq!(world_y_span(&band_at(2, 1)), (0.0, 32.0));
    assert_eq!(world_y_span(&band_at(127, 1)), (96.0, 128.0));
}

#[test]
fn every_coordinate_stays_near_the_origin() {
    for steps in [1, 2] {
        let mut planner = planner(steps);
        planner.update(BlockPos { x: 10_000, y: 70, z: -10_000 }, 0);
        assert!(planner.quads().iter().flat_map(|q| q.corners).flatten().all(|v| v.abs() <= 24.0));
    }
}

#[test]
fn rebuild_only_on_change() {
    let mut planner = planner(1);
    assert!(planner.update(at(5, 5), 0));
    let version = planner.version();
    assert!(!planner.update(at(5, 5), 0));
    assert_eq!(planner.version(), version);
    assert!(!planner.update(at(9, 12), 0));
    assert!(planner.update(at(21, 5), 0));
    assert_eq!(planner.version(), version + 1);
    assert!(!planner.update(BlockPos { x: 21, y: 79, z: 5 }, 0));
    assert!(planner.update(BlockPos { x: 21, y: 80, z: 5 }, 0));
}

#[test]
fn restyle_rebuilds_with_new_color() {
    let mut planner = planner(1);
    planner.update(at(0, 0), 0);
    let version = planner.version();
    let config = BordersConfig { line_color: [1.0, 0.0, 0.0, 0.5], ..BordersConfig::default() };
    planner.restyle(config);
    assert!(planner.update(at(0, 0), 0));
    assert_eq!(planner.version(), version + 1);
    assert!(planner.quads().iter().any(|q| q.rgba == [255, 0, 0, 128]));
}

#[test]
fn buffer_does_not_grow() {
    let mut planner = planner(1);
    planner.update(at(0, 0), 0);
    let single = planner.resident_bytes();
    planner.cycle();
    planner.update(at(0, 0), 0);
    let grid = planner.resident_bytes();
    planner.update(at(0, 0), 1);
    let nether = planner.resident_bytes();
    assert_eq!(single, grid);
    assert_eq!(grid, nether);
}

#[test]
fn grid_in_the_overworld_makes_the_most_quads() {
    let mut planner = planner(2);
    planner.update(at(0, 0), 0);
    assert_eq!(planner.mode(), BorderMode::Grid);
    assert_eq!(planner.quads().len(), ChunkBorderPlanner::max_quads());
}

#[test]
fn no_wall_lies_on_a_block_face() {
    for steps in [1, 2] {
        let mut planner = planner(steps);
        planner.update(at(0, 0), 0);
        let planes = wall_planes(&planner);
        assert!(!planes.is_empty());
        for (_, plane) in planes {
            assert!(!near(plane, plane.round()));
        }
    }
}

#[test]
fn every_wall_is_inset_toward_the_center_chunk() {
    let mut planner = planner(2);
    planner.update(at(0, 0), 0);
    for (_, plane) in wall_planes(&planner) {
        let line = plane.round();
        if line < 0.0 {
            assert!(near(plane, line + INSET));
        } else {
            assert!(near(plane, line - INSET));
        }
    }
}
