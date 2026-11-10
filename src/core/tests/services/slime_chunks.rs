use levels_core::services::SlimeChunks;

#[test]
fn the_reference_vectors_match() {
    assert!(SlimeChunks::is_slime(-1, 0));
    assert!(SlimeChunks::is_slime(109, 3));
    assert!(!SlimeChunks::is_slime(0, 0));
    assert!(!SlimeChunks::is_slime(110, 3));
}

#[test]
fn about_one_chunk_in_ten_is_a_slime_chunk() {
    let n =
        (-50..50).flat_map(|x| (-50..50).map(move |z| (x, z))).filter(|&(x, z)| SlimeChunks::is_slime(x, z)).count();
    assert!((800..=1200).contains(&n), "{n} of 10000");
}
