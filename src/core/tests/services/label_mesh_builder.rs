use levels_core::services::LabelMeshBuilder;
use levels_core::structs::draw::{ColorQuad, Label};

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
    builder.build(&[label(8, [1.0; 4])], 0.0, 1.0);

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
    builder.build(&[label(15, [1.0; 4])], 0.0, 1.0);

    let (x_lo, x_hi) = span(&builder, 0);
    assert!(near(x_hi - x_lo, 0.28));
    assert!(near((x_lo + x_hi) / 2.0, 10.5));
}

#[test]
fn quarter_turn_swaps_the_axes() {
    let mut builder = LabelMeshBuilder::new(4);
    builder.build(&[label(15, [1.0; 4])], 90.0, 1.0);

    let (z_lo, z_hi) = span(&builder, 2);
    let (x_lo, x_hi) = span(&builder, 0);
    assert!(near(z_hi - z_lo, 0.28));
    assert!(near(x_hi - x_lo, 0.2));
}

#[test]
fn scale_multiplies_the_pixel_size() {
    let mut builder = LabelMeshBuilder::new(4);
    builder.build(&[label(15, [1.0; 4])], 0.0, 2.0);

    let (x_lo, x_hi) = span(&builder, 0);
    assert!(near(x_hi - x_lo, 0.56));
}

#[test]
fn colour_is_rounded_to_bytes() {
    let mut builder = LabelMeshBuilder::new(1);
    builder.build(&[label(1, [1.0, 0.5, 0.0, 1.0])], 0.0, 1.0);

    assert_eq!(builder.quads()[0].rgba, [255, 128, 0, 255]);
}

#[test]
fn same_input_keeps_the_version() {
    let mut builder = LabelMeshBuilder::new(4);
    let labels = [label(7, [1.0; 4])];
    let first = builder.build(&labels, 10.0, 1.0);

    assert_eq!(builder.build(&labels, 20.0, 1.0), first);
    assert_eq!(builder.build(&labels, 10.0, 1.0), first);
}

#[test]
fn colour_or_quarter_change_raises_the_version() {
    let mut builder = LabelMeshBuilder::new(4);
    let first = builder.build(&[label(7, [1.0; 4])], 0.0, 1.0);
    let second = builder.build(&[label(7, [0.0, 1.0, 0.0, 1.0])], 0.0, 1.0);
    let third = builder.build(&[label(7, [0.0, 1.0, 0.0, 1.0])], 90.0, 1.0);
    let fourth = builder.build(&[label(7, [0.0, 1.0, 0.0, 1.0])], 90.0, 2.0);

    assert!(first < second && second < third && third < fourth);
}

#[test]
fn quads_never_pass_the_reserved_capacity() {
    let mut builder = LabelMeshBuilder::new(2);
    let labels = [label(10, [1.0; 4]); 3];
    builder.build(&labels, 0.0, 1.0);
    let bytes = builder.resident_bytes();

    assert!(builder.quads().len() <= 2 * LabelMeshBuilder::MAX_QUADS_PER_LABEL);
    assert_eq!(builder.quads().len(), 2 * (5 + 8));
    builder.build(&labels, 90.0, 1.0);
    assert_eq!(builder.resident_bytes(), bytes);
}

#[test]
fn no_labels_gives_zero_origin_and_no_quads() {
    let mut builder = LabelMeshBuilder::new(2);
    builder.build(&[], 0.0, 1.0);

    assert!(builder.quads().is_empty());
    assert_eq!(builder.origin().x, 0);
}
