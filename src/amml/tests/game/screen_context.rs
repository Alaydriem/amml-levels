use std::cell::Cell;
use std::ffi::c_void;

use alaydriem_levels::game::{ScreenContext, ShaderColorLayout};

use crate::support::FakeMeshEntries;

const IDENTITY: [[f32; 4]; 4] =
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

// the matrix times the point (x, y, z, 1), by columns
fn apply(columns: &[[f32; 4]; 4], point: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        columns[0][row] * point[0] + columns[1][row] * point[1] + columns[2][row] * point[2] + columns[3][row]
    })
}

fn assert_near(found: [f32; 3], expected: [f32; 3]) {
    for axis in 0..3 {
        assert!((found[axis] - expected[axis]).abs() < 1e-5, "{found:?} != {expected:?}");
    }
}

#[test]
fn translate_yaw_moves_a_vertex_to_the_offset_plus_the_vertex_turned_about_y_by_degrees() {
    let mut columns = IDENTITY;

    ScreenContext::translate_yaw(&mut columns, [1.0, 2.0, 3.0], 90.0);

    assert_near(apply(&columns, [0.0, 0.0, 0.0]), [1.0, 2.0, 3.0]);
    assert_near(apply(&columns, [1.0, 0.0, 0.0]), [1.0, 2.0, 2.0]);
    assert_near(apply(&columns, [0.0, 0.0, 1.0]), [2.0, 2.0, 3.0]);
    assert_near(apply(&columns, [0.0, 1.0, 0.0]), [1.0, 3.0, 3.0]);
}

#[test]
fn translate_yaw_applies_after_the_matrix_already_on_the_stack() {
    let mut columns = IDENTITY;
    columns[3] = [10.0, 0.0, -5.0, 1.0];

    ScreenContext::translate_yaw(&mut columns, [1.0, 0.0, 0.0], 0.0);

    assert_near(apply(&columns, [0.0, 4.0, 0.0]), [11.0, 4.0, -5.0]);
}

// a fake ScreenContext: the MeshContext base at the fake entry's 0x10, its currentShaderColor pointer 0x20 into it
fn screen_with_color(color: *mut ShaderColorLayout) -> [u64; 16] {
    let mut screen = [0u64; 16];
    screen[(0x10 + 0x20) / 8] = color as u64;
    screen
}

#[test]
fn a_world_draw_sees_opaque_white_and_the_label_colour_comes_back_after() {
    let labels = ShaderColorLayout { color: [0.0; 4], dirty: false };
    let mut color = labels;
    let mut screen = screen_with_color(&mut color);
    let calls = FakeMeshEntries::calls();
    // SAFETY: the buffer is laid out as the fields the wrapper reads and lives for the test
    let context = unsafe { ScreenContext::from_raw(screen.as_mut_ptr().cast::<c_void>(), &calls) }.unwrap();
    let pointer: *const ShaderColorLayout = &color;
    let seen = Cell::new(None);
    // SAFETY: `pointer` is the live colour above
    context.with_white_shader_color(|| seen.set(Some(unsafe { *pointer })));
    assert_eq!(seen.get(), Some(ShaderColorLayout { color: [1.0; 4], dirty: true }));
    assert_eq!(color, ShaderColorLayout { color: labels.color, dirty: true });
}

#[test]
fn a_null_shader_colour_still_draws() {
    let mut screen = screen_with_color(std::ptr::null_mut());
    let calls = FakeMeshEntries::calls();
    // SAFETY: the buffer is laid out as the fields the wrapper reads and lives for the test
    let context = unsafe { ScreenContext::from_raw(screen.as_mut_ptr().cast::<c_void>(), &calls) }.unwrap();
    let drew = Cell::new(false);
    context.with_white_shader_color(|| drew.set(true));
    assert!(drew.get());
}
