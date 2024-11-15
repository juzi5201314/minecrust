use std::borrow::Borrow;

use bevy::prelude::Deref;

/// == `hstr::Atom`, but impl `Borrow<str>`. hashmap::get need it.
#[derive(Deref, Debug, Clone, Hash, PartialEq, Eq)]
pub struct Atom(#[deref] pub hstr::Atom);

impl Atom {
    pub fn new(value: impl Into<hstr::Atom>) -> Self {
        Atom(value.into())
    }
}

impl Borrow<str> for Atom {
    fn borrow(&self) -> &str {
        &self
    }
}

impl<'de> serde::Deserialize<'de> for Atom {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        hstr::Atom::deserialize(deserializer).map(Atom)
    }
}

impl serde::Serialize for Atom {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        self.0.serialize(serializer)
    }
}
