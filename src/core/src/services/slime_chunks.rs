// Bedrock slime chunks: a fixed pattern from the chunk position alone, the same in every world
pub struct SlimeChunks;

impl SlimeChunks {
    const SEED_MULTIPLIER: u32 = 0x1F1F_1F1F;
    const INIT_MULTIPLIER: u32 = 1_812_433_253;
    const MATRIX_A: u32 = 0x9908_B0DF;
    const TEMPER_B: u32 = 0x9D2C_5680;
    const TEMPER_C: u32 = 0xEFC6_0000;
    // the state word that the first MT19937 output reads besides words 0 and 1
    const SHIFT_WORD: u32 = 397;

    // the first MT19937 output seeded from the chunk position; a slime chunk when it is a multiple of 10
    pub fn is_slime(chunk_x: i32, chunk_z: i32) -> bool {
        let seed = (chunk_x as u32).wrapping_mul(Self::SEED_MULTIPLIER) ^ (chunk_z as u32);
        let (mut prev, mut mt1) = (seed, 0u32);
        for i in 1..=Self::SHIFT_WORD {
            prev = Self::INIT_MULTIPLIER.wrapping_mul(prev ^ (prev >> 30)).wrapping_add(i);
            if i == 1 {
                mt1 = prev;
            }
        }
        let y = (seed & 0x8000_0000) | (mt1 & 0x7FFF_FFFF);
        let mut n = prev ^ (y >> 1) ^ if y & 1 == 1 { Self::MATRIX_A } else { 0 };
        n ^= n >> 11;
        n ^= (n << 7) & Self::TEMPER_B;
        n ^= (n << 15) & Self::TEMPER_C;
        n ^= n >> 18;
        n.is_multiple_of(10)
    }
}
