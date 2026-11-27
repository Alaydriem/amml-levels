use amml_api::{ColorQuad, Rgba, Vec3};
use levels_core::structs::draw::ColorQuad as CoreQuad;

// the quads of core as the quads of the loader's canvas
pub struct QuadConverter;

impl QuadConverter {
    // replaces the content of `out` with the converted quads
    pub fn fill(quads: &[CoreQuad], out: &mut Vec<ColorQuad>) {
        out.clear();
        out.extend(quads.iter().map(Self::convert));
    }

    pub fn convert(quad: &CoreQuad) -> ColorQuad {
        ColorQuad {
            corners: quad.corners.map(|corner| Vec3 { x: corner[0], y: corner[1], z: corner[2] }),
            rgba: Rgba { r: quad.rgba[0], g: quad.rgba[1], b: quad.rgba[2], a: quad.rgba[3] },
        }
    }
}
