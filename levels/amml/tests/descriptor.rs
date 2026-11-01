use amm_core::services::DescriptorChecker;
use amm_core::services::settings::SchemaChecker;
use amm_core::structs::descriptor::{Capability, Chord, ClaimMode, KeyScope};
use amm_core::structs::settings::HclDocument;
use levels_core::config::LevelsConfig;

const DESCRIPTOR: &str = include_str!("../descriptor.hcl");
const KEY_G: u32 = 0x47;
const KEY_J: u32 = 0x4A;
const KEY_L: u32 = 0x4C;

fn checker() -> DescriptorChecker {
    DescriptorChecker::new(128, 256)
}

#[test]
fn descriptor_passes_the_manager_checker() {
    let descriptor = checker().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    assert_eq!(descriptor.namespace, "alaydriem");
    assert_eq!(descriptor.name, "levels");
    assert_eq!(descriptor.id(), "@alaydriem/levels");
    let claims: Vec<_> = descriptor.claims.iter().map(|claim| (claim.capability, claim.mode)).collect();
    assert_eq!(claims, [(Capability::WorldDraw, ClaimMode::Shared)]);
    let keybinds: Vec<_> = descriptor
        .keybinds
        .iter()
        .map(|keybind| (keybind.action.as_str(), keybind.capability, keybind.scope, keybind.default_chord))
        .collect();
    let chord = |key: u32, shift: bool, alt: bool| Chord { key, ctrl: false, shift, alt, f3: true };
    let expected = [
        ("toggle", Capability::WorldDraw, KeyScope::Global, chord(KEY_L, false, false)),
        ("borders", Capability::WorldDraw, KeyScope::Global, chord(KEY_G, false, false)),
        ("sphere_drop", Capability::WorldDraw, KeyScope::Global, chord(KEY_J, false, false)),
        ("sphere_remove", Capability::WorldDraw, KeyScope::Global, chord(KEY_J, true, false)),
        ("sphere_clear", Capability::WorldDraw, KeyScope::Global, chord(KEY_J, false, true)),
    ];
    assert_eq!(keybinds, expected);
}

#[test]
fn schema_defaults_match_core_defaults() {
    let descriptor = checker().parse(DESCRIPTOR).expect("the descriptor parses and checks");

    let checked = SchemaChecker::check(&descriptor.settings, HclDocument::empty());
    let config: LevelsConfig = hcl::from_str(&checked.document.text()).expect("the default document parses");

    assert_eq!(config, LevelsConfig::default());
}
