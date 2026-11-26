use core::fmt;

// One fixed-capacity text line with a color. Text past the capacity is dropped.
#[derive(Debug, Clone)]
pub struct InfoLine {
    buf: [u8; Self::MAX_LINE_BYTES],
    len: usize,
    full: bool,
    color: [f32; 3],
}

impl InfoLine {
    pub const MAX_LINE_BYTES: usize = 96;
    const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

    pub fn new() -> Self {
        Self { buf: [0; Self::MAX_LINE_BYTES], len: 0, full: false, color: Self::WHITE }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.full = false;
    }

    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }

    pub fn color(&self) -> [f32; 3] {
        self.color
    }

    pub fn set_color(&mut self, color: [f32; 3]) {
        self.color = color;
    }

    pub(crate) fn reset(&mut self) {
        self.clear();
        self.color = Self::WHITE;
    }
}

impl Default for InfoLine {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Write for InfoLine {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.full {
            return Ok(());
        }
        let room = Self::MAX_LINE_BYTES - self.len;
        let mut cut = s.len().min(room);
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        self.buf[self.len..self.len + cut].copy_from_slice(&s.as_bytes()[..cut]);
        self.len += cut;
        if cut < s.len() {
            self.full = true;
        }
        Ok(())
    }
}
