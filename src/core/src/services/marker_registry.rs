use std::collections::VecDeque;

use crate::structs::marker::SphereMarker;
use crate::structs::world::BlockPos;

// Holds at most `capacity` markers, reserved once. Dropping past capacity evicts the oldest.
pub struct MarkerRegistry {
    capacity: usize,
    markers: VecDeque<SphereMarker>,
    next_id: u32,
}

impl MarkerRegistry {
    pub fn new(capacity: usize) -> Self {
        Self { capacity, markers: VecDeque::with_capacity(capacity), next_id: 1 }
    }

    pub fn drop_at(&mut self, feet: BlockPos, dimension: i32) -> Option<SphereMarker> {
        if self.capacity == 0 {
            return None;
        }
        let evicted = if self.markers.len() >= self.capacity { self.markers.pop_front() } else { None };
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        self.markers.push_back(SphereMarker {
            id,
            center: [feet.x as f32 + 0.5, feet.y as f32, feet.z as f32 + 0.5],
            dimension,
        });
        evicted
    }

    pub fn remove_nearest(&mut self, player: [f32; 3], dimension: i32) -> Option<SphereMarker> {
        let mut best: Option<(usize, f32)> = None;
        for (i, m) in self.markers.iter().enumerate() {
            if m.dimension != dimension {
                continue;
            }
            let d = Self::distance(m.center, player);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.and_then(|(i, _)| self.markers.remove(i))
    }

    pub fn clear(&mut self) {
        self.markers.clear();
    }

    pub fn len(&self) -> usize {
        self.markers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }

    pub fn visible<'a>(
        &'a self,
        player: [f32; 3],
        dimension: i32,
        show_distance: f32,
    ) -> impl Iterator<Item = &'a SphereMarker> + 'a {
        self.markers
            .iter()
            .filter(move |m| m.dimension == dimension && Self::distance(m.center, player) <= show_distance)
    }

    pub fn resident_bytes(&self) -> u64 {
        (self.markers.capacity() * std::mem::size_of::<SphereMarker>()) as u64
    }

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}
