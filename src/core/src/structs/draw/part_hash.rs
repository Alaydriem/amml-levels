use crate::structs::draw::Label;

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

// FNV-1a over the words of the labels of one part. The seed carries what every part shares: the origin, the quarter
// and the scale
#[derive(Debug, Clone, Copy)]
pub struct PartHash {
    seed: u64,
    state: u64,
}

impl PartHash {
    pub fn new(origin: [i32; 3], quarter: u8, scale: f32) -> Self {
        let mut hash = Self { seed: OFFSET, state: OFFSET };
        for word in origin {
            hash.word(word as u32);
        }
        hash.word(quarter as u32);
        hash.word(scale.to_bits());
        hash.seed = hash.state;
        hash
    }

    // starts the hash of the next part
    pub fn restart(&mut self) {
        self.state = self.seed;
    }

    pub fn push(&mut self, label: &Label) {
        self.word(label.x.to_bits());
        self.word(label.y.to_bits());
        self.word(label.z.to_bits());
        self.word(label.value as u32);
        for channel in label.color {
            self.word(channel.to_bits());
        }
    }

    pub fn finish(&self) -> u64 {
        self.state
    }

    fn word(&mut self, word: u32) {
        for byte in word.to_le_bytes() {
            self.state = (self.state ^ byte as u64).wrapping_mul(PRIME);
        }
    }
}
