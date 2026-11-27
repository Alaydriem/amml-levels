use levels_core::services::EntityCensus;
use levels_core::structs::info::ActorSample;
use levels_core::structs::world::Vec3f;

const ZOMBIE: u32 = 0x30B20;
const SKELETON: u32 = 0x110B22;
const CREEPER: u32 = 0xB21;
const SPIDER: u32 = 0x40B23;
const COW: u32 = 0x130B;

struct Fixture;

impl Fixture {
    fn actor(actor_type: u32) -> ActorSample {
        ActorSample { actor_type, ..ActorSample::default() }
    }

    fn local_player() -> ActorSample {
        ActorSample { actor_type: 0x13F, is_local_player: true, ..ActorSample::default() }
    }
}

#[test]
fn local_player_not_counted() {
    let mut c = EntityCensus::new();
    c.submit(&[Fixture::local_player(), Fixture::actor(COW), Fixture::actor(COW)]);
    assert_eq!(c.counts().total, 2);
    assert_eq!(c.counts().players, 0);
    assert_eq!(c.counts().passive, 2);
}

#[test]
fn counts_clear_between_scans() {
    let mut c = EntityCensus::new();
    c.submit(&[Fixture::actor(ZOMBIE), Fixture::actor(ZOMBIE), Fixture::actor(ZOMBIE)]);
    assert_eq!(c.counts().hostile, 3);
    c.submit(&[Fixture::actor(COW)]);
    assert_eq!(c.counts().hostile, 0);
    assert_eq!(c.counts().top_hostile, [(0, 0); 3]);
}

#[test]
fn top_hostile_order_and_ties() {
    let mut c = EntityCensus::new();
    let mut v = Vec::new();
    v.extend([Fixture::actor(SKELETON); 3]);
    v.extend([Fixture::actor(ZOMBIE); 3]);
    v.push(Fixture::actor(CREEPER));
    v.extend([Fixture::actor(SPIDER); 2]);
    c.submit(&v);
    assert_eq!(c.counts().top_hostile, [(0x20, 3), (0x22, 3), (0x23, 2)]);
    assert_eq!(c.counts().hostile, 9);
}

#[test]
fn bound_256() {
    let mut c = EntityCensus::new();
    let mut v = vec![Fixture::actor(COW); 256];
    // the 257th and later actors are hostile and sit at a unique point
    let late = ActorSample {
        min: Vec3f { x: 50.0, y: 50.0, z: 50.0 },
        max: Vec3f { x: 51.0, y: 51.0, z: 51.0 },
        actor_type: ZOMBIE,
        is_local_player: false,
    };
    v.extend([late; 44]);
    c.submit(&v);
    assert_eq!(c.counts().total, 256);
    assert_eq!(c.counts().passive, 256);
    assert_eq!(c.counts().hostile, 0);
    assert_eq!(c.kind_at(Vec3f { x: 50.5, y: 50.5, z: 50.5 }), None);
}

#[test]
fn exception_kind_counts_as_hostile() {
    let mut c = EntityCensus::new();
    c.submit(&[Fixture::actor(0x137C), Fixture::actor(0x137C), Fixture::actor(COW)]);
    assert_eq!(c.counts().hostile, 2);
    assert_eq!(c.counts().passive, 1);
    assert_eq!(c.counts().top_hostile[0], (0x7C, 2));
}

#[test]
fn remote_player_counts() {
    let mut c = EntityCensus::new();
    c.submit(&[Fixture::local_player(), Fixture::actor(0x13F)]);
    assert_eq!(c.counts().players, 1);
    assert_eq!(c.counts().total, 1);
}

#[test]
fn kind_at_finds_box_with_margin() {
    let mut c = EntityCensus::new();
    let z = ActorSample {
        min: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
        max: Vec3f { x: 1.0, y: 2.0, z: 1.0 },
        actor_type: ZOMBIE,
        is_local_player: false,
    };
    c.submit(&[z]);
    assert_eq!(c.kind_at(Vec3f { x: 1.05, y: 1.0, z: 0.5 }), Some(ZOMBIE));
    assert_eq!(c.kind_at(Vec3f { x: 1.5, y: 1.0, z: 0.5 }), None);
}

#[test]
fn kind_at_skips_local_player() {
    let mut c = EntityCensus::new();
    c.submit(&[Fixture::local_player()]);
    assert_eq!(c.kind_at(Vec3f::default()), None);
}
