use std::collections::HashMap;

use levels_core::config::ScanConfig;
use levels_core::services::ScanPlanner;
use levels_core::structs::world::{BlockPos, ColumnPos};

const FRAME: f32 = 1.0 / 60.0;

fn disk(radius: i32) -> usize {
    let mut count = 0;
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            if dx * dx + dz * dz <= radius * radius {
                count += 1;
            }
        }
    }
    count
}

fn feet() -> BlockPos {
    BlockPos { x: 100, y: 64, z: -40 }
}

#[test]
fn total_columns_is_the_disk() {
    let planner = ScanPlanner::new(&ScanConfig::default());

    assert_eq!(planner.total_columns(), disk(24));
}

#[test]
fn one_pass_covers_every_column_once() {
    let mut planner = ScanPlanner::new(&ScanConfig::default());
    let total = planner.total_columns();
    let mut seen: Vec<ColumnPos> = Vec::new();
    let mut frames = 0;
    while seen.len() < total {
        seen.extend(planner.plan(feet(), FRAME).iter().map(|r| r.column));
        frames += 1;
    }

    let mut counts: HashMap<ColumnPos, usize> = HashMap::new();
    for column in &seen[..total] {
        *counts.entry(*column).or_default() += 1;
    }
    assert_eq!(counts.len(), total);
    assert!(counts.values().all(|&n| n == 1));
    assert!(frames <= (0.25_f32 * 60.0).ceil() as usize);
}

#[test]
fn columns_follow_the_player() {
    let mut planner = ScanPlanner::new(&ScanConfig::default());
    planner.plan(feet(), FRAME);
    let moved = BlockPos { x: feet().x + 10, ..feet() };

    for request in planner.plan(moved, FRAME) {
        let dx = request.column.x - moved.x;
        let dz = request.column.z - moved.z;
        assert!(dx * dx + dz * dz <= 24 * 24);
    }
}

#[test]
fn y_range_is_feet_plus_minus_radius() {
    let mut planner = ScanPlanner::new(&ScanConfig::default());

    for request in planner.plan(feet(), FRAME) {
        assert_eq!(request.y_min, feet().y - 24);
        assert_eq!(request.y_max, feet().y + 24);
    }
}

#[test]
fn huge_dt_is_capped() {
    let mut planner = ScanPlanner::new(&ScanConfig::default());

    assert_eq!(planner.plan(feet(), 60.0).len(), 512);
}

#[test]
fn zero_dt_still_samples_one() {
    let mut planner = ScanPlanner::new(&ScanConfig::default());

    assert_eq!(planner.plan(feet(), 0.0).len(), 1);
}
