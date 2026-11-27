use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::ModEvent;
use amm_core::structs::lease::AcquireOutcome;
use amm_core::structs::registry::ModIndex;

use crate::support::levels_rig::{INFO, LevelsRig};
use crate::support::runtime_rig::RuntimeRig;

#[test]
fn a_preempting_hud_grant_holds_the_lease_and_shows_the_panel() {
    let rig = RuntimeRig::new("");
    FakeTable::set_acquire_outcome(Capability::Hud, AcquireOutcome::Preempted);

    rig.press(INFO);

    assert!(rig.info_shown());
    rig.press(INFO);
    assert_eq!(LevelsRig::released(), [Capability::Hud]);
}

#[test]
fn a_stale_hud_suspend_while_released_is_ignored() {
    let rig = RuntimeRig::new("");

    rig.push(ModEvent::Suspended { capability: Capability::Hud, by: ModIndex(2) });
    assert!(LevelsRig::released().is_empty(), "a lease the mod does not hold is not given back");
    rig.press(INFO);

    assert!(rig.info_shown());
    assert_eq!(LevelsRig::acquired(), [Capability::Hud]);
}

#[test]
fn dropping_the_session_releases_a_held_hud_lease() {
    let rig = RuntimeRig::new("");
    rig.press(INFO);
    assert!(LevelsRig::released().is_empty());

    rig.runtime.detach();

    assert_eq!(LevelsRig::released(), [Capability::Hud]);
}
