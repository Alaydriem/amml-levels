use crate::services::ActorKinds;
use crate::structs::info::{ActorClass, ActorSample, EntityCounts};
use crate::structs::world::Vec3f;

pub struct EntityCensus {
    samples: [ActorSample; EntityCensus::CAPACITY],
    len: usize,
    hostile_kinds: [u16; 256],
    counts: EntityCounts,
}

impl EntityCensus {
    // at most this many actors are kept from one scan; the rest are dropped
    pub const CAPACITY: usize = 256;
    pub const HIT_MARGIN: f32 = 0.1;

    pub fn new() -> Self {
        Self {
            samples: [ActorSample::default(); Self::CAPACITY],
            len: 0,
            hostile_kinds: [0; 256],
            counts: EntityCounts::default(),
        }
    }

    pub fn submit(&mut self, actors: &[ActorSample]) {
        self.len = 0;
        self.hostile_kinds = [0; 256];
        self.counts = EntityCounts::default();
        for actor in actors {
            if actor.is_local_player {
                continue;
            }
            if self.len == Self::CAPACITY {
                break;
            }
            self.samples[self.len] = *actor;
            self.len += 1;
            self.counts.total += 1;
            match ActorKinds::classify(actor.actor_type) {
                ActorClass::Player => self.counts.players += 1,
                ActorClass::Hostile => {
                    self.counts.hostile += 1;
                    self.hostile_kinds[(actor.actor_type & 0xFF) as usize] += 1;
                }
                ActorClass::Passive => self.counts.passive += 1,
                ActorClass::Other => self.counts.other += 1,
            }
        }
        self.counts.top_hostile = self.top_hostile();
    }

    pub fn counts(&self) -> &EntityCounts {
        &self.counts
    }

    pub fn kind_at(&self, point: Vec3f) -> Option<u32> {
        let m = Self::HIT_MARGIN;
        self.samples[..self.len]
            .iter()
            .find(|a| {
                point.x >= a.min.x - m
                    && point.x <= a.max.x + m
                    && point.y >= a.min.y - m
                    && point.y <= a.max.y + m
                    && point.z >= a.min.z - m
                    && point.z <= a.max.z + m
            })
            .map(|a| a.actor_type)
    }

    fn top_hostile(&self) -> [(u8, u16); 3] {
        let mut top = [(0u8, 0u16); 3];
        for (kind, &count) in self.hostile_kinds.iter().enumerate() {
            if count == 0 {
                continue;
            }
            // strict comparison keeps the lower kind byte first on a tie
            let Some(slot) = top.iter().position(|&(_, c)| count > c) else {
                continue;
            };
            let mut i = top.len() - 1;
            while i > slot {
                top[i] = top[i - 1];
                i -= 1;
            }
            top[slot] = (kind as u8, count);
        }
        top
    }
}

impl Default for EntityCensus {
    fn default() -> Self {
        Self::new()
    }
}
