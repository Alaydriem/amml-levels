use amml_api::Biomes;

// the biome names the loader gave, by id, for the current world. At most CAPACITY names; a new name past it takes the
// slot of the least recently used one. A biome id can belong to one world, so the names are cleared on a change of
// world or dimension. A name is allocated only when it enters the cache
pub struct BiomeNames {
    // (id, name, the use stamp of its last lookup)
    entries: Vec<(u16, String, u64)>,
    stamp: u64,
}

impl BiomeNames {
    pub const CAPACITY: usize = 128;

    pub fn new() -> Self {
        Self { entries: Vec::with_capacity(Self::CAPACITY), stamp: 0 }
    }

    // the name of biome `id`; None when the loader has no name for it, which is not cached
    pub fn name(&mut self, biomes: &Biomes, id: u16) -> Option<&str> {
        self.stamp += 1;
        let stamp = self.stamp;
        let slot = match self.entries.iter().position(|entry| entry.0 == id) {
            Some(slot) => {
                self.entries[slot].2 = stamp;
                slot
            }
            None => self.insert(id, biomes.name(id)?, stamp),
        };
        Some(self.entries[slot].1.as_str())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    // the slots and the names they hold
    pub fn resident_bytes(&self) -> u64 {
        let slots = self.entries.capacity() * std::mem::size_of::<(u16, String, u64)>();
        let names: usize = self.entries.iter().map(|entry| entry.1.capacity()).sum();
        (slots + names) as u64
    }

    fn insert(&mut self, id: u16, name: String, stamp: u64) -> usize {
        if self.entries.len() < Self::CAPACITY {
            self.entries.push((id, name, stamp));
            return self.entries.len() - 1;
        }
        let oldest = self.entries.iter().enumerate().min_by_key(|(_, entry)| entry.2).map_or(0, |(slot, _)| slot);
        self.entries[oldest] = (id, name, stamp);
        oldest
    }
}

impl Default for BiomeNames {
    fn default() -> Self {
        Self::new()
    }
}
