// A dropped spawn sphere marker. The center is in world coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereMarker {
    pub id: u32,
    pub center: [f32; 3],
    pub dimension: i32,
}
