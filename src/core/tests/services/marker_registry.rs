use levels_core::services::MarkerRegistry;
use levels_core::structs::world::BlockPos;

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

#[test]
fn drop_centers_on_the_block() {
    let mut registry = MarkerRegistry::new(4);
    registry.drop_at(pos(-3, 70, 5), 0);
    let marker = registry.visible([0.0, 70.0, 0.0], 0, 100.0).next().unwrap();
    assert_eq!(marker.center, [-2.5, 70.0, 5.5]);
}

#[test]
fn drop_past_capacity_evicts_the_oldest() {
    let mut registry = MarkerRegistry::new(3);
    assert!(registry.drop_at(pos(0, 64, 0), 0).is_none());
    assert!(registry.drop_at(pos(1, 64, 0), 0).is_none());
    assert!(registry.drop_at(pos(2, 64, 0), 0).is_none());
    let evicted = registry.drop_at(pos(3, 64, 0), 0).unwrap();
    assert_eq!(evicted.id, 1);
    assert_eq!(registry.len(), 3);
    let ids: Vec<u32> = registry.visible([0.0, 64.0, 0.0], 0, 100.0).map(|m| m.id).collect();
    assert_eq!(ids, vec![2, 3, 4]);
}

#[test]
fn remove_nearest_only_in_this_dimension() {
    let mut registry = MarkerRegistry::new(4);
    registry.drop_at(pos(100, 64, 0), 0);
    registry.drop_at(pos(1, 64, 0), 1);
    let removed = registry.remove_nearest([0.5, 64.0, 0.5], 0).unwrap();
    assert_eq!(removed.dimension, 0);
    assert_eq!(removed.center[0], 100.5);
    assert!(registry.remove_nearest([0.5, 64.0, 0.5], 2).is_none());
    assert_eq!(registry.len(), 1);
}

#[test]
fn visible_respects_distance_boundary_and_dimension() {
    let mut registry = MarkerRegistry::new(4);
    // marker center is (0.5, 64, 0.5)
    registry.drop_at(pos(0, 64, 0), 0);
    assert_eq!(registry.visible([48.5, 64.0, 0.5], 0, 48.0).count(), 1);
    assert_eq!(registry.visible([48.51, 64.0, 0.5], 0, 48.0).count(), 0);
    assert_eq!(registry.visible([0.5, 64.0, 0.5], 1, 48.0).count(), 0);
}

#[test]
fn clear_empties() {
    let mut registry = MarkerRegistry::new(4);
    registry.drop_at(pos(0, 64, 0), 0);
    registry.clear();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}
