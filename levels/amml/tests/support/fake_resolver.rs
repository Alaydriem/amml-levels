use std::collections::HashMap;

use amml_game::{EntryId, EntryNeed, EntryResolver, GameEntries};
use amml_sdk::SdkError;

// a resolver that knows the ids it was given
pub struct FakeResolver {
    known: HashMap<&'static str, u64>,
}

impl FakeResolver {
    pub fn new(values: &[(EntryId, u64)]) -> Self {
        Self { known: values.iter().map(|(id, value)| (id.as_str(), *value)).collect() }
    }

    // entries with exactly these values; every other id is missing
    pub fn entries(values: &[(EntryId, u64)]) -> GameEntries {
        let wanted: Vec<_> = values.iter().map(|(id, _)| (*id, EntryNeed::Optional { feature: "test" })).collect();
        GameEntries::resolve(&Self::new(values), &wanted).unwrap()
    }
}

impl EntryResolver for FakeResolver {
    fn resolve(&self, id: &str) -> Result<u64, SdkError> {
        self.known.get(id).copied().ok_or_else(|| SdkError::MissingEntry(id.to_string()))
    }
}
