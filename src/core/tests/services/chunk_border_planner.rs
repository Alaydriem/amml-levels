use levels_core::config::BordersConfig;
use levels_core::services::{ChunkBorderPlanner, SlimeChunks};
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
fn vibrant_rebuilds_with_thinner_and_fainter_lines_once() {
    let config = BordersConfig { line_color: [1.0, 0.0, 0.0, 0.5], ..BordersConfig::default() };
    let mut planner = ChunkBorderPlanner::new(config.clone());
    planner.cycle();
    planner.update(at(0, 0), 0);
    let version = planner.version();
    let widest = |planner: &ChunkBorderPlanner| {
        planner
            .quads()
            .iter()
            .filter(|q| is_horizontal(q))
            .map(|q| (q.corners[2][1] - q.corners[0][1]).abs())
            .fold(0.0, f32::max)
    };
    let normal_width = widest(&planner);

    planner.set_vibrant(true);
    assert!(planner.update(at(0, 0), 0));
    planner.set_vibrant(true);
    assert!(!planner.update(at(0, 0), 0));

    assert_eq!(planner.version(), version + 1);
    assert!(planner.quads().iter().any(|q| q.rgba == [255, 0, 0, 64]));
    assert!(planner.quads().iter().all(|q| q.rgba != [255, 0, 0, 128]));
    assert!(near(widest(&planner), normal_width * ChunkBorderPlanner::VIBRANT_WIDTH_SCALE));
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

const SLIME_RGB: [u8; 3] = [77, 255, 77];

fn slime_planner(steps: usize, highlight: bool, feet: BlockPos) -> ChunkBorderPlanner {
    let config = BordersConfig { slime_highlight: highlight, ..BordersConfig::default() };
    let mut planner = ChunkBorderPlanner::new(config);
    for _ in 0..steps {
        planner.cycle();
    }
    planner.update(feet, 0);
    planner
}

// true when a chunk inside the drawn grid that touches the quad is a slime chunk
fn touches_slime(planner: &ChunkBorderPlanner, quad: &ColorQuad, feet: BlockPos, radius: i32) -> bool {
    let origin = planner.origin();
    let mid = |axis: usize| quad.corners.iter().map(|c| c[axis]).sum::<f32>() / 4.0;
    let (x, z) = (mid(0) + origin.x as f32, mid(2) + origin.z as f32);
    let fixed_is_x = quad.corners.iter().all(|c| c[0] == quad.corners[0][0]);
    let (fixed, along) = if fixed_is_x { (x, z) } else { (z, x) };
    let on_chunk_line = !is_horizontal(quad) && near(along, along.round()) && along.round() as i32 % 16 == 0;
    let along_offsets: &[f32] = if on_chunk_line { &[-0.5, 0.5] } else { &[0.0] };
    let (cx, cz) = (feet.x.div_euclid(16), feet.z.div_euclid(16));
    let in_grid = |chunk_x: i32, chunk_z: i32| (chunk_x - cx).abs() <= radius && (chunk_z - cz).abs() <= radius;
    [-0.5f32, 0.5].iter().any(|df| {
        along_offsets.iter().any(|da| {
            let a = ((fixed.round() + df).floor() as i32).div_euclid(16);
            let b = ((along + da).floor() as i32).div_euclid(16);
            let (chunk_x, chunk_z) = if fixed_is_x { (a, b) } else { (b, a) };
            in_grid(chunk_x, chunk_z) && SlimeChunks::is_slime(chunk_x, chunk_z)
        })
    })
}

#[test]
fn single_mode_in_a_slime_chunk_is_all_green() {
    let feet = at(-8, 8);
    let planner = slime_planner(1, true, feet);
    assert!(planner.quads().iter().all(|q| q.rgba[..3] == SLIME_RGB));
}

#[test]
fn single_mode_outside_a_slime_chunk_has_no_green() {
    let feet = at(8, 8);
    let planner = slime_planner(1, true, feet);
    assert!(planner.quads().iter().all(|q| q.rgba[..3] != SLIME_RGB));
}

#[test]
fn grid_segments_are_green_when_a_grid_chunk_beside_them_is_slime() {
    for feet in [at(8, 8), at(-8, 8), at(1750, 50), at(-300, -700)] {
        let planner = slime_planner(2, true, feet);
        let mut greens = 0;
        for quad in planner.quads() {
            let expected = touches_slime(&planner, quad, feet, 1);
            assert_eq!(quad.rgba[..3] == SLIME_RGB, expected, "at {feet:?}, quad {quad:?}");
            greens += usize::from(expected);
        }
        if feet == at(8, 8) {
            assert!(greens > 0);
        }
    }
}

#[test]
fn slime_chunks_keep_the_original_colors_when_highlight_is_off() {
    for steps in [1, 2] {
        let planner = slime_planner(steps, false, at(-8, 8));
        assert!(planner.quads().iter().all(|q| q.rgba[..3] != SLIME_RGB));
    }
}

#[test]
fn turning_highlight_on_rebuilds_the_mesh() {
    let mut planner = slime_planner(1, false, at(-8, 8));
    let version = planner.version();
    planner.restyle(BordersConfig { slime_highlight: true, ..BordersConfig::default() });
    assert!(planner.update(at(-8, 8), 0));
    assert_eq!(planner.version(), version + 1);
    assert!(planner.quads().iter().all(|q| q.rgba[..3] == SLIME_RGB));
}

#[test]
fn edge_lines_keep_the_edge_opacity_in_green() {
    let planner = slime_planner(1, true, at(-8, 8));
    let edge_alpha = (BordersConfig::default().edge_alpha * 255.0).round() as u8;
    assert!(planner.quads().iter().any(|q| q.rgba == [77, 255, 77, edge_alpha]));
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
