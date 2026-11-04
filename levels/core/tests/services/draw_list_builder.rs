use std::collections::HashSet;

use levels_core::config::{LabelConfig, LightConfig, ScanConfig};
use levels_core::services::{DrawListBuilder, LabelGrid, LightClassifier};
use levels_core::structs::draw::Label;
use levels_core::structs::light::SurfaceSample;
use levels_core::structs::world::{BlockPos, ColumnPos};

fn builder(scan: &ScanConfig) -> DrawListBuilder {
    DrawListBuilder::new(scan, LabelConfig::default(), LightClassifier::new(LightConfig::default()))
}

fn sample(x: i32, y: i32, z: i32, block_light: u8, sky_light: u8) -> SurfaceSample {
    SurfaceSample { pos: BlockPos { x, y, z }, block_light, sky_light }
}

fn feet() -> BlockPos {
    BlockPos { x: 0, y: 64, z: 0 }
}

// the block each label sits on, as (x, y)
fn blocks(labels: &[Label]) -> HashSet<(i32, i32)> {
    labels.iter().map(|l| ((l.x - 0.5).round() as i32, (l.y - 1.02).round() as i32)).collect()
}

#[test]
fn sphere_limits_labels() {
    let scan = ScanConfig::default();
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: 0, z: 0 }, &[sample(0, 40, 0, 0, 0), sample(0, 88, 0, 0, 0), sample(0, 89, 0, 0, 0)]);
    grid.store(ColumnPos { x: 24, z: 0 }, &[sample(24, 64, 0, 0, 0), sample(24, 65, 0, 0, 0)]);

    let mut builder = builder(&scan);
    let labels = builder.build(&grid, feet(), 0);

    assert_eq!(blocks(labels), HashSet::from([(0, 40), (0, 88), (24, 64)]));
}

#[test]
fn label_is_on_the_top_face() {
    let scan = ScanConfig::default();
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: 3, z: -2 }, &[sample(3, 64, -2, 9, 0)]);

    let mut builder = builder(&scan);
    let labels = builder.build(&grid, feet(), 0);

    assert_eq!(labels.len(), 1);
    assert!((labels[0].x - 3.5).abs() < 1e-5);
    assert!((labels[0].y - 65.02).abs() < 1e-4);
    assert!((labels[0].z - -1.5).abs() < 1e-5);
}

#[test]
fn colors_follow_the_classifier() {
    let scan = ScanConfig::default();
    let colors = LabelConfig::default();
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: 1, z: 0 }, &[sample(1, 64, 0, 0, 0)]);
    grid.store(ColumnPos { x: 2, z: 0 }, &[sample(2, 64, 0, 5, 0)]);
    grid.store(ColumnPos { x: 3, z: 0 }, &[sample(3, 64, 0, 12, 0)]);

    let mut builder = builder(&scan);
    let mut labels = builder.build(&grid, feet(), 0).to_vec();
    labels.sort_by(|a, b| a.x.total_cmp(&b.x));

    let drawn: Vec<(u8, [f32; 4])> = labels.iter().map(|l| (l.value, l.color)).collect();
    assert_eq!(drawn, vec![(0, colors.red), (5, colors.yellow), (12, colors.green)]);
}

#[test]
fn value_uses_time_of_day() {
    let scan = ScanConfig::default();
    let colors = LabelConfig::default();
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: 0, z: 0 }, &[sample(0, 63, 0, 0, 15)]);
    let mut builder = builder(&scan);

    let noon = builder.build(&grid, feet(), 0)[0];
    let night = builder.build(&grid, feet(), 11)[0];
    let dark = builder.build(&grid, feet(), 15)[0];

    assert_eq!((noon.value, noon.color), (15, colors.green));
    assert_eq!((night.value, night.color), (4, colors.yellow));
    assert_eq!((dark.value, dark.color), (0, colors.red));
}

#[test]
fn cap_keeps_nearest() {
    let scan = ScanConfig { max_labels: 10, ..ScanConfig::default() };
    let mut grid = LabelGrid::new(scan.radius);
    for x in 0..25 {
        grid.store(ColumnPos { x, z: 0 }, &[sample(x, 64, 0, 0, 0), sample(x, 65, 0, 0, 0)]);
    }

    let mut builder = builder(&scan);
    let labels = builder.build(&grid, feet(), 0);

    let nearest: HashSet<(i32, i32)> = (0..5).flat_map(|x| [(x, 64), (x, 65)]).collect();
    assert_eq!(labels.len(), 10);
    assert_eq!(blocks(labels), nearest);
}

#[test]
fn same_set_in_any_insert_order_gives_the_same_list() {
    let scan = ScanConfig { max_labels: 3, ..ScanConfig::default() };
    let columns = [(3, 64), (-3, 64), (1, 67), (-1, 61), (4, 66), (-4, 66)];
    let mut forward = LabelGrid::new(scan.radius);
    let mut backward = LabelGrid::new(scan.radius);
    for &(x, y) in &columns {
        forward.store(ColumnPos { x, z: 0 }, &[sample(x, y, 0, 3, 0)]);
    }
    for &(x, y) in columns.iter().rev() {
        backward.store(ColumnPos { x, z: 0 }, &[sample(x, y, 0, 3, 0)]);
    }
    let mut builder = builder(&scan);

    let first = builder.build(&forward, feet(), 0).to_vec();
    let second = builder.build(&backward, feet(), 0).to_vec();

    assert_eq!(first.len(), 3);
    assert_eq!(first, second);
    let xs: Vec<f32> = first.iter().map(|l| l.x).collect();
    assert!(xs.windows(2).all(|pair| pair[0] <= pair[1]));
}
