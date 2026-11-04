use levels_core::services::SphereShellBuilder;
use levels_core::structs::draw::ColorQuad;

fn length(c: [f32; 3]) -> f32 {
    (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt()
}

#[test]
fn fill_is_on_the_radius_and_lines_sit_the_lift_outside_it() {
    let builder = SphereShellBuilder::new();
    let mut out = Vec::new();
    let fill_alpha = (0.25_f32 * 255.0).round() as u8;
    for radius in [24.0_f32, 44.5] {
        builder.build(radius, [1.0, 0.0, 0.0, 0.25], &mut out);
        for quad in &out {
            let target = if quad.rgba[3] == fill_alpha { radius } else { radius + SphereShellBuilder::LINE_LIFT };
            for c in quad.corners {
                let len = length(c);
                assert!((len - target).abs() <= 0.01, "len {len} target {target}");
            }
        }
    }
}

#[test]
fn without_fill_only_the_line_quads_are_built_and_turning_it_back_on_restores_them() {
    let mut builder = SphereShellBuilder::new();
    let mut quads = Vec::new();
    assert!(builder.set_fill(false));
    assert!(!builder.set_fill(false));

    builder.build(24.0, [0.0, 1.0, 0.0, 0.08], &mut quads);

    let fill = (SphereShellBuilder::SEGMENTS * SphereShellBuilder::RINGS) as usize;
    assert_eq!(quads.len(), SphereShellBuilder::quad_count() - fill);
    assert!(quads.iter().all(|quad| quad.rgba[3] > (0.08f32 * 255.0).round() as u8));
    assert!(builder.set_fill(true));
    builder.build(24.0, [0.0, 1.0, 0.0, 0.08], &mut quads);
    assert_eq!(quads.len(), SphereShellBuilder::quad_count());
}

#[test]
fn quad_count_matches() {
    let builder = SphereShellBuilder::new();
    let mut out: Vec<ColorQuad> = Vec::new();
    builder.build(24.0, [0.0, 1.0, 0.0, 0.25], &mut out);
    assert_eq!(out.len(), SphereShellBuilder::quad_count());
}

#[test]
fn line_quads_are_stronger() {
    let builder = SphereShellBuilder::new();
    let mut out = Vec::new();
    builder.build(24.0, [0.0, 0.0, 1.0, 0.08], &mut out);
    let fill = (0.08_f32 * 255.0).round() as u8;
    let line = (0.08_f32 * 6.0 * 255.0).round() as u8;
    let stronger: Vec<_> = out.iter().filter(|q| q.rgba[3] > fill).collect();
    assert!(!stronger.is_empty());
    assert!(stronger.iter().all(|q| q.rgba[3] == line));

    builder.build(24.0, [0.0, 0.0, 1.0, 0.02], &mut out);
    let floor = (0.35_f32 * 255.0).round() as u8;
    let faint = (0.02_f32 * 255.0).round() as u8;
    let lines: Vec<_> = out.iter().filter(|q| q.rgba[3] > faint).collect();
    assert!(!lines.is_empty());
    assert!(lines.iter().all(|q| q.rgba[3] == floor));
}

#[test]
fn build_reuses_the_buffer() {
    let builder = SphereShellBuilder::new();
    let mut out = Vec::with_capacity(SphereShellBuilder::quad_count());
    builder.build(24.0, [1.0, 1.0, 1.0, 0.25], &mut out);
    let capacity = out.capacity();
    builder.build(44.5, [1.0, 1.0, 1.0, 0.25], &mut out);
    assert_eq!(out.capacity(), capacity);
    assert_eq!(out.len(), SphereShellBuilder::quad_count());
}
