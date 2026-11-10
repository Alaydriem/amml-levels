use crate::structs::draw::ColorQuad;

// Builds the quads of one sphere shell, local to the sphere center: the translucent fill, then the grid lines, or
// the lines alone. The caller owns the output buffer, which `build` clears and refills and `append` extends.
pub struct SphereShellBuilder {
    with_fill: bool,
}

impl SphereShellBuilder {
    pub const SEGMENTS: u32 = 32;
    pub const RINGS: u32 = 16;
    pub const LINE_EVERY: u32 = 4;
    const LINE_WIDTH: f32 = 0.15;
    // the lines sit this far outside the fill: the game keeps mesh positions as 16-bit floats (a 0.03-block step at a
    // 44.5-block radius), and the depth buffer cannot split two surfaces much closer than that at 100 blocks
    pub const LINE_LIFT: f32 = 0.1;
    const LINE_ALPHA_GAIN: f32 = 6.0;
    const LINE_ALPHA_MIN: f32 = 0.35;

    // a builder that draws the fill and the lines
    pub fn new() -> Self {
        Self { with_fill: true }
    }

    // sets whether the fill is drawn; true when that changes the quads
    pub fn set_fill(&mut self, with_fill: bool) -> bool {
        let changed = self.with_fill != with_fill;
        self.with_fill = with_fill;
        changed
    }

    pub const fn quad_count() -> usize {
        let fill = (Self::SEGMENTS * Self::RINGS) as usize;
        let meridians = (Self::SEGMENTS / Self::LINE_EVERY) as usize;
        let parallels = ((Self::RINGS - 1) / Self::LINE_EVERY) as usize;
        fill + meridians * Self::RINGS as usize + parallels * Self::SEGMENTS as usize
    }

    pub const fn pair_quad_count() -> usize {
        2 * Self::quad_count()
    }

    pub fn build(&self, radius: f32, color: [f32; 4], out: &mut Vec<ColorQuad>) {
        out.clear();
        self.append(radius, color, out);
    }

    pub fn append(&self, radius: f32, color: [f32; 4], out: &mut Vec<ColorQuad>) {
        let fill = Self::bytes(color[0], color[1], color[2], color[3]);
        let line = Self::bytes(
            color[0],
            color[1],
            color[2],
            (color[3] * Self::LINE_ALPHA_GAIN).clamp(Self::LINE_ALPHA_MIN, 1.0),
        );

        for ring in (0..Self::RINGS).filter(|_| self.with_fill) {
            for seg in 0..Self::SEGMENTS {
                let corners = [
                    Self::point(radius, seg, ring),
                    Self::point(radius, seg + 1, ring),
                    Self::point(radius, seg + 1, ring + 1),
                    Self::point(radius, seg, ring + 1),
                ];
                out.push(ColorQuad { corners, rgba: fill });
            }
        }

        let lifted = radius + Self::LINE_LIFT;
        let half = Self::LINE_WIDTH * 0.5;

        for seg in (0..Self::SEGMENTS).step_by(Self::LINE_EVERY as usize) {
            for ring in 0..Self::RINGS {
                let west_top = Self::offset(lifted, seg, ring, -half, 0.0);
                let east_top = Self::offset(lifted, seg, ring, half, 0.0);
                let east_bottom = Self::offset(lifted, seg, ring + 1, half, 0.0);
                let west_bottom = Self::offset(lifted, seg, ring + 1, -half, 0.0);
                out.push(ColorQuad { corners: [west_top, east_top, east_bottom, west_bottom], rgba: line });
            }
        }

        for ring in (Self::LINE_EVERY..Self::RINGS).step_by(Self::LINE_EVERY as usize) {
            for seg in 0..Self::SEGMENTS {
                let a = Self::offset(lifted, seg, ring, 0.0, -half);
                let b = Self::offset(lifted, seg + 1, ring, 0.0, -half);
                let c = Self::offset(lifted, seg + 1, ring, 0.0, half);
                let d = Self::offset(lifted, seg, ring, 0.0, half);
                out.push(ColorQuad { corners: [a, b, c, d], rgba: line });
            }
        }
    }

    fn bytes(r: f32, g: f32, b: f32, a: f32) -> [u8; 4] {
        [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8, (a * 255.0).round() as u8]
    }

    // unit direction, then the east and south tangents at a grid point
    fn frame(seg: u32, ring: u32) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let theta = seg as f32 / Self::SEGMENTS as f32 * std::f32::consts::TAU;
        let phi = ring as f32 / Self::RINGS as f32 * std::f32::consts::PI;
        let (st, ct) = theta.sin_cos();
        let (sp, cp) = phi.sin_cos();
        ([sp * ct, cp, sp * st], [-st, 0.0, ct], [cp * ct, -sp, cp * st])
    }

    fn point(radius: f32, seg: u32, ring: u32) -> [f32; 3] {
        let (dir, _, _) = Self::frame(seg, ring);
        [dir[0] * radius, dir[1] * radius, dir[2] * radius]
    }

    // grid point pushed along the east and south tangents, then put back on the sphere
    fn offset(radius: f32, seg: u32, ring: u32, east: f32, south: f32) -> [f32; 3] {
        let (dir, e, s) = Self::frame(seg, ring);
        let p = [
            dir[0] * radius + e[0] * east + s[0] * south,
            dir[1] * radius + e[1] * east + s[1] * south,
            dir[2] * radius + e[2] * east + s[2] * south,
        ];
        let len = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        let k = radius / len;
        [p[0] * k, p[1] * k, p[2] * k]
    }
}

impl Default for SphereShellBuilder {
    fn default() -> Self {
        Self::new()
    }
}
