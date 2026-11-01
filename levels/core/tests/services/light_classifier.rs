use levels_core::config::LightConfig;
use levels_core::services::LightClassifier;
use levels_core::structs::light::LightBand;

#[test]
fn default_bands_follow_the_users_rule() {
    let classifier = LightClassifier::new(LightConfig::default());

    assert_eq!(classifier.classify(0), LightBand::Red);
    assert_eq!(classifier.classify(1), LightBand::Yellow);
    assert_eq!(classifier.classify(7), LightBand::Yellow);
    assert_eq!(classifier.classify(8), LightBand::Green);
    assert_eq!(classifier.classify(15), LightBand::Green);
}

#[test]
fn configured_limits_move_the_bands() {
    let classifier = LightClassifier::new(LightConfig { red_max: 2, yellow_max: 10 });

    assert_eq!(classifier.classify(2), LightBand::Red);
    assert_eq!(classifier.classify(3), LightBand::Yellow);
    assert_eq!(classifier.classify(10), LightBand::Yellow);
    assert_eq!(classifier.classify(11), LightBand::Green);
}

#[test]
fn light_takes_brighter_of_block_and_darkened_sky() {
    assert_eq!(LightClassifier::light(0, 15, 0), 15);
    assert_eq!(LightClassifier::light(0, 15, 11), 4);
    assert_eq!(LightClassifier::light(14, 15, 11), 14);
    assert_eq!(LightClassifier::light(3, 15, 15), 3);
    assert_eq!(LightClassifier::light(0, 4, 11), 0);
    assert_eq!(LightClassifier::light(20, 0, 0), 15);
}
