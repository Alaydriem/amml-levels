use alaydriem_levels::game::WireframeMaterial;
use amml_game::GameEntries;

use crate::support::{FakeMaterialChain, FakeResolver};

fn read(chain: &FakeMaterialChain, entries: &GameEntries) -> Option<WireframeMaterial> {
    // SAFETY: the fake chain is live memory laid out by the entries
    unsafe { WireframeMaterial::from_level_renderer(chain.level_renderer(), entries) }
}

#[test]
fn from_level_renderer_follows_the_chain_to_the_render_material() {
    let chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();

    let material = read(&chain, &entries).unwrap();

    assert_eq!(material.material_ptr() as usize, chain.player_address() + FakeMaterialChain::MATERIAL_AT);
    assert_eq!(material.render_material().as_ptr() as usize, chain.material_address());
}

#[test]
fn from_level_renderer_is_none_on_each_null_hop() {
    let entries = FakeMaterialChain::entries();
    let mut no_player = FakeMaterialChain::new();
    no_player.null_player();
    let mut no_info = FakeMaterialChain::new();
    no_info.null_info();
    let mut no_material = FakeMaterialChain::new();
    no_material.null_material();

    assert!(read(&no_player, &entries).is_none());
    assert!(read(&no_info, &entries).is_none());
    assert!(read(&no_material, &entries).is_none());
    // SAFETY: a null LevelRenderer is rejected before any read
    assert!(unsafe { WireframeMaterial::from_level_renderer(std::ptr::null(), &entries) }.is_none());
}

#[test]
fn from_level_renderer_is_none_when_an_entry_is_missing() {
    let chain = FakeMaterialChain::new();
    let mut values = FakeMaterialChain::values();
    values.remove(1);
    let entries = FakeResolver::entries(&values);

    assert!(read(&chain, &entries).is_none());
}
