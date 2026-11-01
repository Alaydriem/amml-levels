use alaydriem_levels::game::MeshLayoutKind;
use alaydriem_levels::types::LevelsEntry;

use crate::support::FakeResolver;

#[test]
fn selects_2660_by_size() {
    let entries = FakeResolver::entries(&[(LevelsEntry::MESH_SIZE, 608)]);

    assert_eq!(MeshLayoutKind::of(&entries), Some(MeshLayoutKind::Build2660));
}

#[test]
fn selects_2651_by_size() {
    let entries = FakeResolver::entries(&[(LevelsEntry::MESH_SIZE, 632)]);

    assert_eq!(MeshLayoutKind::of(&entries), Some(MeshLayoutKind::Build2651));
}

#[test]
fn unknown_size_selects_none() {
    for size in [0, 600, 616, 640] {
        let entries = FakeResolver::entries(&[(LevelsEntry::MESH_SIZE, size)]);

        assert_eq!(MeshLayoutKind::of(&entries), None, "{size}");
    }
    assert_eq!(MeshLayoutKind::of(&FakeResolver::entries(&[])), None);
}
