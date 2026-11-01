use levels_core::structs::world::DimensionRange;

#[test]
fn known_dimensions_use_their_build_limits() {
    assert_eq!(DimensionRange::for_dimension(0), DimensionRange { min_y: -64, max_y: 320 });
    assert_eq!(DimensionRange::for_dimension(1), DimensionRange { min_y: 0, max_y: 128 });
    assert_eq!(DimensionRange::for_dimension(2), DimensionRange { min_y: 0, max_y: 256 });
}

#[test]
fn unknown_dimension_uses_overworld_range() {
    assert_eq!(DimensionRange::for_dimension(7), DimensionRange::for_dimension(0));
    assert_eq!(DimensionRange::for_dimension(-3), DimensionRange::for_dimension(0));
}
