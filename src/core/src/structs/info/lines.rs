use super::InfoLine;

// Fixed set of lines, reused each refresh.
#[derive(Debug, Clone)]
pub struct InfoLines {
    lines: Vec<InfoLine>,
    len: usize,
}

impl InfoLines {
    pub const MAX_LINES: usize = 24;

    pub fn new() -> Self {
        Self { lines: vec![InfoLine::new(); Self::MAX_LINES], len: 0 }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn next_line(&mut self) -> Option<&mut InfoLine> {
        let line = self.lines.get_mut(self.len)?;
        self.len += 1;
        line.reset();
        Some(line)
    }

    pub fn iter(&self) -> impl Iterator<Item = &InfoLine> {
        self.lines[..self.len].iter()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl Default for InfoLines {
    fn default() -> Self {
        Self::new()
    }
}
