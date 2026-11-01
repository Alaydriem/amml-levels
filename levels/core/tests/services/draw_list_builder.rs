use std::collections::HashSet;

use levels_core::config::{GAME_TAG_HEADROOM, LabelConfig, LightConfig, ScanConfig};
use levels_core::services::{DrawListBuilder, LabelGrid, LightClassifier};
use levels_core::structs::draw::{CameraView, Label};
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
    let labels = builder.build(&grid, feet(), 0, 0, None);

    assert_eq!(blocks(labels), HashSet::from([(0, 40), (0, 88), (24, 64)]));
}

#[test]
fn label_is_on_the_top_face() {
    let scan = ScanConfig::default();
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: 3, z: -2 }, &[sample(3, 64, -2, 9, 0)]);

    let mut builder = builder(&scan);
    let labels = builder.build(&grid, feet(), 0, 0, None);

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
    let mut labels = builder.build(&grid, feet(), 0, 0, None).to_vec();
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

    let noon = builder.build(&grid, feet(), 0, 0, None)[0];
    let night = builder.build(&grid, feet(), 11, 0, None)[0];
    let dark = builder.build(&grid, feet(), 15, 0, None)[0];

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
    let labels = builder.build(&grid, feet(), 0, 0, None);

    let nearest: HashSet<(i32, i32)> = (0..5).flat_map(|x| [(x, 64), (x, 65)]).collect();
    assert_eq!(labels.len(), 10);
    assert_eq!(blocks(labels), nearest);
}

#[test]
fn game_tags_over_the_headroom_lower_the_cap_and_keep_the_nearest() {
    let scan = ScanConfig { max_labels: 10, ..ScanConfig::default() };
    let mut grid = LabelGrid::new(scan.radius);
    for x in 0..25 {
        grid.store(ColumnPos { x, z: 0 }, &[sample(x, 64, 0, 0, 0), sample(x, 65, 0, 0, 0)]);
    }
    let mut builder = builder(&scan);

    let within_headroom = builder.build(&grid, feet(), 0, GAME_TAG_HEADROOM, None).len();
    let labels = builder.build(&grid, feet(), 0, GAME_TAG_HEADROOM + 4, None);

    let nearest: HashSet<(i32, i32)> = (0..3).flat_map(|x| [(x, 64), (x, 65)]).collect();
    assert_eq!(within_headroom, 10);
    assert_eq!(labels.len(), 6);
    assert_eq!(blocks(labels), nearest);
}

#[test]
fn game_tags_beyond_every_label_leave_none() {
    let scan = ScanConfig { max_labels: 10, ..ScanConfig::default() };
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: 0, z: 0 }, &[sample(0, 64, 0, 0, 0)]);
    let mut builder = builder(&scan);

    assert!(builder.build(&grid, feet(), 0, GAME_TAG_HEADROOM + 50, None).is_empty());
}

#[test]
fn labels_behind_the_camera_are_not_drawn() {
    let scan = ScanConfig::default();
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: 5, z: 0 }, &[sample(5, 63, 0, 0, 0)]);
    grid.store(ColumnPos { x: -5, z: 0 }, &[sample(-5, 63, 0, 0, 0)]);
    grid.store(ColumnPos { x: 0, z: 6 }, &[sample(0, 63, 6, 0, 0)]);
    let camera = CameraView::from_forward([0.5, 65.6, 0.5], [1.0, 0.0, 0.0]);
    let mut builder = builder(&scan);

    let labels = builder.build(&grid, feet(), 0, 0, camera);

    assert_eq!(blocks(labels), HashSet::from([(5, 63), (0, 63)]));
    assert_eq!(builder.culled(), 1);
}

#[test]
fn culled_count_is_per_build() {
    let scan = ScanConfig::default();
    let mut grid = LabelGrid::new(scan.radius);
    grid.store(ColumnPos { x: -5, z: 0 }, &[sample(-5, 63, 0, 0, 0)]);
    let camera = CameraView::from_forward([0.5, 65.6, 0.5], [1.0, 0.0, 0.0]);
    let mut builder = builder(&scan);
    builder.build(&grid, feet(), 0, 0, camera);

    builder.build(&grid, feet(), 0, 0, None);

    assert_eq!(builder.culled(), 0);
}

#[test]
fn culling_happens_before_the_cap() {
    let scan = ScanConfig { max_labels: 3, ..ScanConfig::default() };
    let mut grid = LabelGrid::new(scan.radius);
    for x in -10..=10 {
        grid.store(ColumnPos { x, z: 0 }, &[sample(x, 63, 0, 0, 0)]);
    }
    let camera = CameraView::from_forward([2.5, 65.6, 0.5], [1.0, 0.0, 0.0]);
    let mut builder = builder(&scan);

    let labels = builder.build(&grid, feet(), 0, 0, camera);

    assert_eq!(blocks(labels), HashSet::from([(1, 63), (2, 63), (3, 63)]));
}
