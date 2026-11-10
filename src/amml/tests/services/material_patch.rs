use alaydriem_levels::game::WireframeMaterial;
use alaydriem_levels::services::MaterialPatch;
use alaydriem_levels::types::LevelsEntry;
use amml_game::GameEntries;

use crate::support::{FakeMaterialChain, FakeResolver};

const BLEND: usize = FakeMaterialChain::BLEND_AT;
const DEPTH: usize = FakeMaterialChain::DEPTH_AT;
const SOURCE: usize = FakeMaterialChain::SOURCE_AT;
const DESTINATION: usize = FakeMaterialChain::DESTINATION_AT;

fn material(chain: &FakeMaterialChain, entries: &GameEntries) -> WireframeMaterial {
    // SAFETY: the fake chain is live memory laid out by the entries
    unsafe { WireframeMaterial::from_level_renderer(chain.level_renderer(), entries) }.unwrap()
}

#[test]
fn apply_sets_blend_on_and_depth_write_off() {
    let chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();

    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    assert!(patch.is_applied());
    assert_eq!(chain.byte(BLEND), 1);
    assert_eq!(chain.byte(DEPTH), 0);
}

#[test]
fn apply_sets_the_alpha_blend_factors_and_restore_puts_the_game_factors_back() {
    let chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();

    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };
    assert_eq!((chain.byte(SOURCE), chain.byte(DESTINATION)), (6, 8));

    // SAFETY: as above
    unsafe { patch.restore_if_current(Some(&material(&chain, &entries)), &entries) };
    assert_eq!((chain.byte(SOURCE), chain.byte(DESTINATION)), (3, 2));
}

#[test]
fn apply_without_a_blend_factor_entry_writes_nothing() {
    let chain = FakeMaterialChain::new();
    let values: Vec<_> =
        FakeMaterialChain::values().into_iter().filter(|(id, _)| *id != LevelsEntry::MATERIAL_BLEND_SOURCE).collect();
    let entries = FakeResolver::entries(&values);
    let mut patch = MaterialPatch::new();

    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    assert!(!patch.is_applied());
    assert_eq!((chain.byte(BLEND), chain.byte(SOURCE), chain.byte(DEPTH)), (0, 3, 1));
}

#[test]
fn restore_if_current_writes_the_original_bytes_once() {
    let mut chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();
    // SAFETY: the material is the live fake
    unsafe {
        patch.apply(&material(&chain, &entries), &entries);
        patch.restore_if_current(Some(&material(&chain, &entries)), &entries);
    }
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (0, 1));
    assert!(!patch.is_applied());

    chain.set_byte(BLEND, 9);
    chain.set_byte(DEPTH, 9);
    // SAFETY: nothing is kept, so nothing is written
    unsafe { patch.restore_if_current(Some(&material(&chain, &entries)), &entries) };

    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (9, 9));
}

#[test]
fn apply_again_after_a_reload_rewrites() {
    let mut chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();
    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    chain.set_byte(BLEND, 0);
    chain.set_byte(DEPTH, 1);
    // SAFETY: as above
    unsafe { patch.apply(&material(&chain, &entries), &entries) };
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (1, 0));

    // SAFETY: as above
    unsafe { patch.restore_if_current(Some(&material(&chain, &entries)), &entries) };
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (0, 1));
}

#[test]
fn apply_to_a_new_material_leaves_the_old_one_untouched() {
    let mut chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();
    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    chain.swap_material();
    // SAFETY: the new material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    assert_eq!((chain.retired_byte(BLEND), chain.retired_byte(DEPTH)), (1, 0));
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (1, 0));
    assert!(patch.is_applied());
    // SAFETY: the current material is the live fake
    unsafe { patch.restore_if_current(Some(&material(&chain, &entries)), &entries) };
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (0, 1));
    assert_eq!((chain.retired_byte(BLEND), chain.retired_byte(DEPTH)), (1, 0));
}

#[test]
fn restore_if_current_skips_a_stale_address() {
    let mut chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();
    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    chain.swap_material();
    // SAFETY: the new material is the live fake
    unsafe { patch.restore_if_current(Some(&material(&chain, &entries)), &entries) };

    assert!(!patch.is_applied());
    assert_eq!((chain.retired_byte(BLEND), chain.retired_byte(DEPTH)), (1, 0));
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (0, 1));
}

#[test]
fn restore_if_current_with_no_material_writes_nothing_and_forgets() {
    let chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();
    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    // SAFETY: there is no current material, so nothing is written
    unsafe { patch.restore_if_current(None, &entries) };

    assert!(!patch.is_applied());
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (1, 0));
}

#[test]
fn forget_writes_nothing() {
    let chain = FakeMaterialChain::new();
    let entries = FakeMaterialChain::entries();
    let mut patch = MaterialPatch::new();
    // SAFETY: the material is the live fake
    unsafe { patch.apply(&material(&chain, &entries), &entries) };

    patch.forget();

    assert!(!patch.is_applied());
    assert_eq!((chain.byte(BLEND), chain.byte(DEPTH)), (1, 0));
}
