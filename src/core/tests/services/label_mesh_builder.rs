use levels_core::services::LabelMeshBuilder;
use levels_core::structs::draw::{ColorQuad, Label};
use levels_core::structs::world::BlockPos;

const ORIGIN: BlockPos = BlockPos { x: 10, y: 64, z: 10 };

fn label(value: u8, color: [f32; 4]) -> Label {
    Label { x: 10.5, y: 64.02, z: 10.5, value, color }
}

fn span(builder: &LabelMeshBuilder, axis: usize) -> (f32, f32) {
    let origin = builder.origin();
    let base = [origin.x, origin.y, origin.z][axis] as f32;
    let values = builder.quads().iter().flat_map(|q: &ColorQuad| q.corners.iter().map(move |c| c[axis] + base));
    values.fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)))
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn quarter_for_yaw_snaps_and_wraps() {
    for (yaw, quarter) in [(0.0, 0), (44.9, 0), (90.0, 1), (-90.0, 3), (359.9, 0), (405.0, 1), (45.0, 1), (-45.0, 3)] {
        assert_eq!(LabelMeshBuilder::quarter_for_yaw(yaw), quarter, "yaw {yaw}");
    }
}

#[test]
fn one_digit_label_is_centred_on_the_block() {
    let mut builder = LabelMeshBuilder::new(4);
    builder.build(&[label(8, [1.0; 4])], ORIGIN, 0.0, 1.0);

    assert_eq!(builder.quads().len(), 7);
    let (x_lo, x_hi) = span(&builder, 0);
    let (z_lo, z_hi) = span(&builder, 2);
    assert!(near(x_lo, 10.44) && near(x_hi, 10.56), "x {x_lo}..{x_hi}");
    assert!(near(z_lo, 10.4) && near(z_hi, 10.6), "z {z_lo}..{z_hi}");
    let (y_lo, y_hi) = span(&builder, 1);
    assert!(near(y_lo, 64.02) && near(y_hi, 64.02));
}

#[test]
fn two_digit_label_is_seven_pixels_wide() {
    let mut builder = LabelMeshBuilder::new(4);
    builder.build(&[label(15, [1.0; 4])], ORIGIN, 0.0, 1.0);

    let (x_lo, x_hi) = span(&builder, 0);
    assert!(near(x_hi - x_lo, 0.28));
    assert!(near((x_lo + x_hi) / 2.0, 10.5));
}

#[test]
fn quarter_turn_swaps_the_axes() {
    let mut builder = LabelMeshBuilder::new(4);
    builder.build(&[label(15, [1.0; 4])], ORIGIN, 90.0, 1.0);

    let (z_lo, z_hi) = span(&builder, 2);
    let (x_lo, x_hi) = span(&builder, 0);
    assert!(near(z_hi - z_lo, 0.28));
    assert!(near(x_hi - x_lo, 0.2));
}

#[test]
fn scale_multiplies_the_pixel_size() {
    let mut builder = LabelMeshBuilder::new(4);
    builder.build(&[label(15, [1.0; 4])], ORIGIN, 0.0, 2.0);

    let (x_lo, x_hi) = span(&builder, 0);
    assert!(near(x_hi - x_lo, 0.56));
}

#[test]
fn colour_is_rounded_to_bytes() {
    let mut builder = LabelMeshBuilder::new(1);
    builder.build(&[label(1, [1.0, 0.5, 0.0, 1.0])], ORIGIN, 0.0, 1.0);

    assert_eq!(builder.quads()[0].rgba, [255, 128, 0, 255]);
}

#[test]
fn same_input_keeps_the_version() {
    let mut builder = LabelMeshBuilder::new(4);
    let labels = [label(7, [1.0; 4])];
    let first = builder.build(&labels, ORIGIN, 10.0, 1.0);

    assert_eq!(builder.build(&labels, ORIGIN, 20.0, 1.0), first);
    assert_eq!(builder.build(&labels, ORIGIN, 10.0, 1.0), first);
}

#[test]
fn colour_or_quarter_change_raises_the_version() {
    let mut builder = LabelMeshBuilder::new(4);
    let first = builder.build(&[label(7, [1.0; 4])], ORIGIN, 0.0, 1.0);
    let second = builder.build(&[label(7, [0.0, 1.0, 0.0, 1.0])], ORIGIN, 0.0, 1.0);
    let third = builder.build(&[label(7, [0.0, 1.0, 0.0, 1.0])], ORIGIN, 90.0, 1.0);
    let fourth = builder.build(&[label(7, [0.0, 1.0, 0.0, 1.0])], ORIGIN, 90.0, 2.0);

    assert!(first < second && second < third && third < fourth);
}

#[test]
fn quads_never_pass_the_reserved_capacity() {
    let mut builder = LabelMeshBuilder::new(2);
    let labels = [label(10, [1.0; 4]); 3];
    builder.build(&labels, ORIGIN, 0.0, 1.0);
    let bytes = builder.resident_bytes();

    assert!(builder.quads().len() <= 2 * LabelMeshBuilder::MAX_QUADS_PER_LABEL);
    assert_eq!(builder.quads().len(), 2 * (5 + 8));
    builder.build(&labels, ORIGIN, 90.0, 1.0);
    assert_eq!(builder.resident_bytes(), bytes);
}

#[test]
fn no_labels_gives_no_quads_and_keeps_the_origin() {
    let mut builder = LabelMeshBuilder::new(2);
    builder.build(&[], ORIGIN, 0.0, 1.0);

    assert!(builder.quads().is_empty());
    assert_eq!(builder.origin(), ORIGIN);
    assert!(builder.parts().is_empty());
}

const CELL: f32 = LabelMeshBuilder::CELL_BLOCKS as f32;

fn at(x: f32, value: u8) -> Label {
    at_z(x, 10.5, value)
}

fn at_z(x: f32, z: f32, value: u8) -> Label {
    Label { x, y: 64.02, z, color: [1.0; 4], value }
}

fn keys(builder: &LabelMeshBuilder) -> Vec<u64> {
    builder.parts().iter().map(|part| part.key).collect()
}

#[test]
fn parts_follow_the_cells_and_cover_every_quad() {
    let mut builder = LabelMeshBuilder::new(8);
    // sorted by position, as the draw list gives them: two cells along x, then a third cell along z in the first column
    let labels = [at_z(1.5, 1.5, 8), at_z(1.5, CELL + 1.5, 15), at_z(CELL + 1.5, 1.5, 8), at_z(CELL + 2.5, 2.5, 8)];
    builder.build(&labels, ORIGIN, 0.0, 1.0);

    let parts = builder.parts();
    assert_eq!(parts.len(), 3);
    assert_eq!((parts[0].start, parts[0].end), (0, 7));
    assert_eq!((parts[1].start, parts[1].end), (7, 17));
    assert_eq!((parts[2].start, parts[2].end), (17, builder.quads().len()));
    assert_eq!(builder.quads().len(), 7 + 10 + 7 + 7);
}

#[test]
fn a_changed_label_changes_only_the_key_of_its_cell() {
    let mut builder = LabelMeshBuilder::new(8);
    let labels = [at(1.5, 8), at(CELL + 1.5, 8), at(2.0 * CELL + 1.5, 8)];
    builder.build(&labels, ORIGIN, 0.0, 1.0);
    let before = keys(&builder);

    let mut changed = labels;
    changed[1] = at(CELL + 1.5, 7);
    builder.build(&changed, ORIGIN, 0.0, 1.0);
    let after = keys(&builder);

    assert_eq!((before[0], before[2]), (after[0], after[2]));
    assert_ne!(before[1], after[1]);
}

#[test]
fn a_label_dropped_from_one_cell_leaves_the_other_keys() {
    let mut builder = LabelMeshBuilder::new(8);
    builder.build(&[at(1.5, 8), at(CELL + 1.5, 8), at(2.0 * CELL + 1.5, 8)], ORIGIN, 0.0, 1.0);
    let before = keys(&builder);

    builder.build(&[at(CELL + 1.5, 8), at(2.0 * CELL + 1.5, 8)], ORIGIN, 0.0, 1.0);

    assert_eq!(keys(&builder), before[1..]);
}

#[test]
fn a_label_added_in_the_middle_of_the_list_leaves_the_keys_of_other_cells() {
    let mut builder = LabelMeshBuilder::new(8);
    builder.build(&[at_z(1.5, 1.5, 8), at_z(CELL + 1.5, 1.5, 8), at_z(CELL + 1.5, CELL + 1.5, 8)], ORIGIN, 0.0, 1.0);
    let before = keys(&builder);

    builder.build(
        &[at_z(1.5, 1.5, 8), at_z(1.5, CELL + 1.5, 8), at_z(CELL + 1.5, 1.5, 8), at_z(CELL + 1.5, CELL + 1.5, 8)],
        ORIGIN,
        0.0,
        1.0,
    );

    let after = keys(&builder);
    assert_eq!(after.len(), 4);
    assert!(before.iter().all(|key| after.contains(key)));
}

#[test]
fn quarter_scale_and_origin_change_every_key() {
    let mut builder = LabelMeshBuilder::new(8);
    let labels = [at(1.5, 8), at(CELL + 1.5, 8)];
    builder.build(&labels, ORIGIN, 0.0, 1.0);
    let base = keys(&builder);

    builder.build(&labels, ORIGIN, 90.0, 1.0);
    let turned = keys(&builder);
    builder.build(&labels, ORIGIN, 0.0, 2.0);
    let scaled = keys(&builder);
    builder.build(&labels, BlockPos { x: 26, ..ORIGIN }, 0.0, 1.0);
    let moved = keys(&builder);

    for other in [turned, scaled, moved] {
        assert!(other.iter().all(|key| !base.contains(key)));
    }
}

#[test]
fn a_cell_with_more_labels_than_a_part_holds_is_split() {
    let capacity = LabelMeshBuilder::MAX_LABELS_PER_PART + 10;
    let mut builder = LabelMeshBuilder::new(capacity);
    let labels: Vec<Label> = (0..capacity).map(|i| at(1.0 + (i as f32) / 1000.0, 8)).collect();
    builder.build(&labels, ORIGIN, 0.0, 1.0);

    let parts = builder.parts();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].end - parts[0].start, LabelMeshBuilder::MAX_LABELS_PER_PART * 7);
    assert_eq!(parts[1].end - parts[1].start, 10 * 7);
}

#[test]
fn past_the_parts_cap_labels_are_dropped_and_no_part_passes_the_mesh_limit() {
    let capacity = 3000;
    let mut builder = LabelMeshBuilder::new(capacity);
    let bytes = builder.resident_bytes();
    // a spread far wider than any label grid, so every cell would start a part
    let labels: Vec<Label> = (0..capacity).map(|i| at(i as f32 * CELL, 8)).collect();

    builder.build(&labels, ORIGIN, 0.0, 1.0);

    assert!(builder.parts().len() <= LabelMeshBuilder::MAX_PARTS);
    assert!(builder.quads().len() < capacity * 7);
    assert!(
        builder.parts().iter().all(|part| part.end - part.start
            <= LabelMeshBuilder::MAX_LABELS_PER_PART * LabelMeshBuilder::MAX_QUADS_PER_LABEL)
    );
    assert_eq!(builder.parts().last().map(|part| part.end), Some(builder.quads().len()));
    assert_eq!(builder.resident_bytes(), bytes);
}

#[test]
fn a_full_grid_of_labels_stays_within_the_parts_of_its_cells_and_label_limit() {
    let capacity = 3000;
    let mut builder = LabelMeshBuilder::new(capacity);
    let labels: Vec<Label> =
        (0..capacity).map(|i| at_z((i % 65) as f32 + 0.5, (i / 65 * 65 / 47) as f32 + 0.5, 8)).collect();

    builder.build(&labels, ORIGIN, 0.0, 1.0);

    let cells = 64 / LabelMeshBuilder::CELL_BLOCKS as usize + 2;
    assert!(builder.parts().len() <= cells * cells + capacity.div_ceil(LabelMeshBuilder::MAX_LABELS_PER_PART));
    assert_eq!(builder.quads().len(), capacity * 7);
}
