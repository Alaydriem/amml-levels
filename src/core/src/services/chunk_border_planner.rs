use crate::config::BordersConfig;
use crate::services::SlimeChunks;
use crate::structs::draw::ColorQuad;
use crate::structs::marker::BorderMode;
use crate::structs::world::{BlockPos, DimensionRange};

const CHUNK: i32 = 16;
// the border covers the player's 16-block section and BAND_MARGIN blocks under and over it
const BAND_MARGIN: i32 = 8;
const BAND_HEIGHT: i32 = CHUNK + 2 * BAND_MARGIN;
const GRID_RADIUS: i32 = 1;
const GRID_SIZE: usize = (2 * GRID_RADIUS + 1) as usize;
const SLIME_COLOR: [f32; 4] = [0.3, 1.0, 0.3, 0.6];
// walls sit this far inside the center chunk so they never share a plane with a block face
const WALL_INSET: f32 = 0.02;

// Builds the translucent grid quads of the chunk borders around the player, in a band of BAND_HEIGHT blocks around
// the player's section. The quads are relative to the centre of the band, so every coordinate stays within 24
// blocks of the origin: the game keeps mesh positions as 16-bit floats, which drop a 0.04-block line past that.
// The quad buffer is reserved once for the Grid mode and reused.
pub struct ChunkBorderPlanner {
    config: BordersConfig,
    mode: BorderMode,
    quads: Vec<ColorQuad>,
    origin: BlockPos,
    built: Option<(i32, i32, BorderMode, i32, i32)>,
    stale: bool,
    version: u64,
    // under Vibrant Visuals overlapping translucent lines build up over frames, so they are drawn thinner and fainter
    vibrant: bool,
    // slime[x][z] for the chunks of the current build, indexed from its north-west chunk
    slime: [[bool; GRID_SIZE]; GRID_SIZE],
}

impl ChunkBorderPlanner {
    // the line widths and alphas under Vibrant Visuals, as a share of the configured ones
    pub const VIBRANT_WIDTH_SCALE: f32 = 0.5;
    pub const VIBRANT_ALPHA_SCALE: f32 = 0.5;

    pub fn new(config: BordersConfig) -> Self {
        Self {
            config,
            mode: BorderMode::Off,
            quads: Vec::with_capacity(Self::max_quads()),
            origin: BlockPos::default(),
            built: None,
            stale: false,
            version: 0,
            vibrant: false,
            slime: [[false; GRID_SIZE]; GRID_SIZE],
        }
    }

    pub fn cycle(&mut self) {
        self.mode = self.mode.next();
        if self.mode == BorderMode::Off {
            self.clear();
        }
    }

    pub fn mode(&self) -> BorderMode {
        self.mode
    }

    pub fn reset(&mut self) {
        self.mode = BorderMode::Off;
        self.clear();
    }

    pub fn restyle(&mut self, config: BordersConfig) {
        if self.config != config {
            self.config = config;
            self.stale = true;
        }
    }

    // sets the graphics mode; a change rebuilds the quads with its widths and alphas
    pub fn set_vibrant(&mut self, vibrant: bool) {
        if self.vibrant != vibrant {
            self.vibrant = vibrant;
            self.stale = true;
        }
    }

    pub fn update(&mut self, feet: BlockPos, dimension: i32) -> bool {
        let radius = match self.mode {
            BorderMode::Off => return false,
            BorderMode::Single => 0,
            BorderMode::Grid => GRID_RADIUS,
        };
        let cx = feet.x.div_euclid(CHUNK);
        let cz = feet.z.div_euclid(CHUNK);
        let section = feet.y.div_euclid(CHUNK);
        let key = (cx, cz, self.mode, dimension, section);
        if !self.stale && self.built == Some(key) {
            return false;
        }

        let band = Self::band(section, DimensionRange::for_dimension(dimension));
        let half_width = (2 * radius + 1) * CHUNK / 2;
        let half_height = (band.max_y - band.min_y) / 2;
        self.origin = BlockPos {
            x: (cx - radius) * CHUNK + half_width,
            y: band.min_y + half_height,
            z: (cz - radius) * CHUNK + half_width,
        };
        self.quads.clear();
        self.mark_slime(cx, cz, radius);
        self.build(radius, band);
        self.center(half_width as f32, half_height as f32);
        self.built = Some(key);
        self.stale = false;
        self.version += 1;
        true
    }

    pub fn quads(&self) -> &[ColorQuad] {
        &self.quads
    }

    pub fn origin(&self) -> BlockPos {
        self.origin
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn resident_bytes(&self) -> u64 {
        (self.quads.capacity() * std::mem::size_of::<ColorQuad>()) as u64
    }

    fn clear(&mut self) {
        self.quads.clear();
        self.built = None;
        self.stale = false;
        self.version += 1;
    }

    // the most quads a build makes: the Grid mode with a full band
    pub const fn max_quads() -> usize {
        Self::quad_capacity(GRID_RADIUS, BAND_HEIGHT as usize)
    }

    // the band around `section`, moved inside the dimension's build range
    fn band(section: i32, range: DimensionRange) -> DimensionRange {
        let height = BAND_HEIGHT.min(range.max_y - range.min_y);
        let min_y = (section * CHUNK - BAND_MARGIN).clamp(range.min_y, range.max_y - height);
        DimensionRange { min_y, max_y: min_y + height }
    }

    // moves the quads from the corner of the box to its centre
    fn center(&mut self, half_width: f32, half_height: f32) {
        let shift = [half_width, half_height, half_width];
        for quad in &mut self.quads {
            for corner in &mut quad.corners {
                for (value, by) in corner.iter_mut().zip(shift) {
                    *value -= by;
                }
            }
        }
    }

    const fn quad_capacity(radius: i32, span_y: usize) -> usize {
        let size = (2 * radius + 1) as usize;
        let walls = size + 1;
        let length = CHUNK as usize * size;
        let horizontal = 2 * walls * size * (span_y + 1);
        let vertical = walls * (length + 1) + walls * (length + 1 - walls);
        horizontal + vertical
    }

    fn build(&mut self, radius: i32, range: DimensionRange) {
        let size = 2 * radius + 1;
        let height = (range.max_y - range.min_y) as f32;
        for fixed_is_x in [true, false] {
            for wall in 0..=size {
                for segment in 0..size {
                    self.push_horizontals(fixed_is_x, wall, segment, radius, height);
                }
                self.push_verticals(fixed_is_x, wall, size, radius, height);
            }
        }
    }

    fn push_horizontals(&mut self, fixed_is_x: bool, wall: i32, segment: i32, radius: i32, height: f32) {
        let half = self.width(self.config.line_width) / 2.0;
        let fixed = Self::wall_plane(wall, radius);
        let lo = (segment * CHUNK) as f32;
        let hi = lo + CHUNK as f32;
        let center = Self::is_center(radius, wall, segment * CHUNK, (segment + 1) * CHUNK);
        let slime = self.touches_slime(fixed_is_x, wall, segment, segment);
        let rgba = Self::pack(self.faded(self.base_color(center, slime)));
        for y in 0..=(height as i32) {
            let y_lo = (y as f32 - half).max(0.0);
            let y_hi = (y as f32 + half).min(height);
            self.quads.push(ColorQuad {
                corners: [
                    Self::point(fixed_is_x, fixed, lo, y_lo),
                    Self::point(fixed_is_x, fixed, hi, y_lo),
                    Self::point(fixed_is_x, fixed, hi, y_hi),
                    Self::point(fixed_is_x, fixed, lo, y_hi),
                ],
                rgba,
            });
        }
    }

    fn push_verticals(&mut self, fixed_is_x: bool, wall: i32, size: i32, radius: i32, height: f32) {
        let fixed = Self::wall_plane(wall, radius);
        let length = size * CHUNK;
        for along in 0..=length {
            let is_edge = along % CHUNK == 0;
            // a z-wall position on a chunk line is already emitted by the x-wall there
            if !fixed_is_x && is_edge {
                continue;
            }
            let width = self.width(if is_edge { self.config.edge_width } else { self.config.line_width });
            let center = Self::is_center(radius, wall, along, along);
            let segment = along / CHUNK;
            let slime = if is_edge {
                self.touches_slime(fixed_is_x, wall, segment - 1, segment)
            } else {
                self.touches_slime(fixed_is_x, wall, segment, segment)
            };
            let mut color = self.base_color(center, slime);
            if is_edge {
                color[3] = self.config.edge_alpha;
            }
            let color = self.faded(color);
            let lo = (along as f32 - width / 2.0).max(0.0);
            let hi = (along as f32 + width / 2.0).min(length as f32);
            self.quads.push(ColorQuad {
                corners: [
                    Self::point(fixed_is_x, fixed, lo, 0.0),
                    Self::point(fixed_is_x, fixed, hi, 0.0),
                    Self::point(fixed_is_x, fixed, hi, height),
                    Self::point(fixed_is_x, fixed, lo, height),
                ],
                rgba: Self::pack(color),
            });
        }
    }

    fn mark_slime(&mut self, cx: i32, cz: i32, radius: i32) {
        let highlight = self.config.slime_highlight;
        let size = (2 * radius + 1) as usize;
        for (ix, column) in self.slime.iter_mut().enumerate() {
            for (iz, cell) in column.iter_mut().enumerate() {
                let inside = ix < size && iz < size;
                *cell = highlight && inside && SlimeChunks::is_slime(cx - radius + ix as i32, cz - radius + iz as i32);
            }
        }
    }

    // true when a slime chunk of the build lies on either side of `wall` between segments `from` and `to`; chunks
    // outside the build count as not slime
    fn touches_slime(&self, fixed_is_x: bool, wall: i32, from: i32, to: i32) -> bool {
        (wall - 1..=wall).any(|side| {
            (from..=to).any(
                |segment| {
                    if fixed_is_x { self.slime_at(side, segment) } else { self.slime_at(segment, side) }
                },
            )
        })
    }

    fn slime_at(&self, x: i32, z: i32) -> bool {
        match (usize::try_from(x), usize::try_from(z)) {
            (Ok(x), Ok(z)) if x < GRID_SIZE && z < GRID_SIZE => self.slime[x][z],
            _ => false,
        }
    }

    // a configured width in the current graphics mode
    fn width(&self, configured: f32) -> f32 {
        if self.vibrant { configured * Self::VIBRANT_WIDTH_SCALE } else { configured }
    }

    // a colour with the alpha of the current graphics mode
    fn faded(&self, mut color: [f32; 4]) -> [f32; 4] {
        if self.vibrant {
            color[3] *= Self::VIBRANT_ALPHA_SCALE;
        }
        color
    }

    fn base_color(&self, center: bool, slime: bool) -> [f32; 4] {
        if slime {
            SLIME_COLOR
        } else if center {
            self.config.center_color
        } else {
            self.config.line_color
        }
    }

    // the player is always inside the center chunk, so every wall is seen from the center side
    fn wall_plane(wall: i32, radius: i32) -> f32 {
        let line = (wall * CHUNK) as f32;
        if wall <= radius { line + WALL_INSET } else { line - WALL_INSET }
    }

    // true when the span lies on one of the four edges of the center chunk in Grid mode
    fn is_center(radius: i32, wall: i32, from: i32, to: i32) -> bool {
        radius > 0 && (wall == radius || wall == radius + 1) && from >= CHUNK * radius && to <= CHUNK * (radius + 1)
    }

    fn pack(color: [f32; 4]) -> [u8; 4] {
        color.map(|c| (c * 255.0).round() as u8)
    }

    fn point(fixed_is_x: bool, fixed: f32, along: f32, y: f32) -> [f32; 3] {
        if fixed_is_x { [fixed, y, along] } else { [along, y, fixed] }
    }
}
