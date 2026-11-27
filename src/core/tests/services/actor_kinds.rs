use levels_core::services::ActorKinds;
use levels_core::structs::info::ActorClass;

#[test]
fn classifies_by_family_and_exceptions() {
    assert_eq!(ActorKinds::classify(0x13F), ActorClass::Player);
    assert_eq!(ActorKinds::classify(0x30B20), ActorClass::Hostile);
    assert_eq!(ActorKinds::classify(0x137C), ActorClass::Hostile);
    assert_eq!(ActorKinds::classify(0x17B), ActorClass::Hostile);
    assert_eq!(ActorKinds::classify(0x17F), ActorClass::Hostile);
    assert_eq!(ActorKinds::classify(0x130B), ActorClass::Passive);
    assert_eq!(ActorKinds::classify(0x40), ActorClass::Other);
    assert_eq!(ActorKinds::classify(0xC00050), ActorClass::Other);
}

#[test]
fn unknown_kind_has_no_name() {
    assert_eq!(ActorKinds::name(0x5C), None);
    assert_eq!(ActorKinds::name(0x30B20), Some("zombie"));
    assert_eq!(ActorKinds::name(0xC00050), Some("arrow"));
}

#[test]
fn first_middle_and_last_entries_resolve() {
    assert_eq!(ActorKinds::name(0x0A), Some("chicken"));
    assert_eq!(ActorKinds::name(0x7C), Some("hoglin"));
    assert_eq!(ActorKinds::name(0xDA), Some("chest_boat"));
}
