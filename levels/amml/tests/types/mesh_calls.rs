use alaydriem_levels::errors::LayoutFault;
use alaydriem_levels::types::{LevelsEntry, MeshCalls};
use amml_game::{CameraLayout, EntryId};

use crate::support::{FakeMeshEntries, FakeResolver};

fn without(id: EntryId) -> Vec<(EntryId, u64)> {
    FakeMeshEntries::values().into_iter().filter(|(entry, _)| *entry != id).collect()
}

fn with(id: EntryId, value: u64) -> Vec<(EntryId, u64)> {
    FakeMeshEntries::values().into_iter().map(|(entry, old)| (entry, if entry == id { value } else { old })).collect()
}

#[test]
fn new_names_a_missing_entry() {
    let result = MeshCalls::new(&FakeResolver::entries(&without(LevelsEntry::MATRIX_STACK_PUSH)));

    assert_eq!(result.err(), Some(LayoutFault::Missing { entry: "MatrixStack.push" }));
}

#[test]
fn new_refuses_anchors_that_are_not_the_mirror_offsets() {
    let cases = [
        (LevelsEntry::TESSELLATOR_TESSELLATING, 0x2A8, 0x2A0),
        (EntryId::CAMERA_FORWARD, CameraLayout::FORWARD + 8, CameraLayout::FORWARD),
        (LevelsEntry::RESOURCE_LOCATION_FULL_HASH, 0x38, 0x30),
        (LevelsEntry::IMAGE_BUFFER_DESCRIPTION, 0x20, 0x18),
        (LevelsEntry::IMAGE_DESCRIPTION_DEPTH, 0x14, 0x10),
        (LevelsEntry::SCREEN_CONTEXT_CAMERA, 0x20, 0x18),
        (LevelsEntry::SCREEN_CONTEXT_CONSTANT_BUFFERS, 0x28, 0x20),
        (LevelsEntry::SCREEN_CONTEXT_CURRENT_SHADER_COLOR, 0x38, 0x30),
        (LevelsEntry::MESH_SIZE, 616, 632),
    ];
    for (id, found, expected) in cases {
        let result = MeshCalls::new(&FakeResolver::entries(&with(id, found)));

        assert_eq!(result.err(), Some(LayoutFault::Misfit { entry: id.as_str(), expected, found }), "{id:?}");
    }
}

#[test]
fn the_mesh_context_members_follow_a_moved_mesh_context_base() {
    let moved = [
        (LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT, 0x18),
        (LevelsEntry::SCREEN_CONTEXT_CAMERA, 0x20),
        (LevelsEntry::SCREEN_CONTEXT_CONSTANT_BUFFERS, 0x28),
        (LevelsEntry::SCREEN_CONTEXT_CURRENT_SHADER_COLOR, 0x38),
    ];
    let values: Vec<_> = FakeMeshEntries::values()
        .into_iter()
        .map(|(entry, old)| (entry, moved.iter().find(|(id, _)| *id == entry).map_or(old, |(_, value)| *value)))
        .collect();

    let calls = MeshCalls::new(&FakeResolver::entries(&values)).unwrap();

    assert_eq!(calls.mesh_context, 0x18);
    let stale = with(LevelsEntry::SCREEN_CONTEXT_MESH_CONTEXT, 0x18);
    let result = MeshCalls::new(&FakeResolver::entries(&stale));
    assert_eq!(
        result.err(),
        Some(LayoutFault::Misfit { entry: "ScreenContext.camera", expected: 0x20, found: 0x18 }),
        "a camera that did not move with the base does not fit"
    );
}
