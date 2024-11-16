use std::borrow::{Borrow, Cow};
use std::fmt::{Debug, Display};
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use once_cell::sync::Lazy;
use scc::HashCache;

pub type Atom = Atom2;

/* /// == `hstr::Atom`, but impl `Borrow<str>`. hashmap::get need it.
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
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
} */

#[derive(Clone)]
pub struct Atom2(Arc<Cell>);

impl Atom2 {
    pub fn new(s: impl Into<Self>) -> Self {
        s.into()
    }

    /* pub fn cached<'a>(s: impl Into<Cow<'a, str>>) -> Self {
        CACHED_STORE.atom(s.into())
    } */

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0.s
    }
}

impl Eq for Atom2 {}
impl PartialEq for Atom2 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq<&str> for Atom2 {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl AsRef<str> for Atom2 {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Deref for Atom2 {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl Borrow<str> for Atom2 {
    fn borrow(&self) -> &str {
        &self
    }
}

impl<'a> From<&'a str> for Atom2 {
    fn from(value: &'a str) -> Self {
        GLOBAL_STORE.atom(Cow::Borrowed(value))
    }
}

impl From<String> for Atom2 {
    fn from(value: String) -> Self {
        GLOBAL_STORE.atom(Cow::Owned(value))
    }
}

//static GLOBAL_STORE: Lazy<GlobalStore> = Lazy::new(|| GlobalStore::default());
static GLOBAL_STORE: Lazy<CachedStore> = Lazy::new(|| CachedStore {
    cache: HashCache::with_capacity_and_hasher(1024, 4096, BuildHasherDefault::default()),
});

#[derive(Default)]
struct GlobalStore {
    store: scc::HashIndex<Arc<Cell>, (), BuildHasherDefault<DirectHasher>>,
}

#[derive(Default)]
struct CachedStore {
    cache: scc::HashCache<Arc<Cell>, (), BuildHasherDefault<DirectHasher>>,
}

struct Cell {
    s: Box<str>,
    hash: u64,
}

struct HashKey {
    hash: u64,
}

impl GlobalStore {
    fn atom(&self, s: Cow<'_, str>) -> Atom2 {
        let hash = calc_hash(&s);
        let hash_key = HashKey { hash };
        let cell = self
            .store
            .peek_with(&hash_key, |cell, _| cell.clone())
            .unwrap_or_else(|| {
                let cell = Arc::new(Cell {
                    s: s.into_owned().into_boxed_str(),
                    hash,
                });
                self.store.entry(cell).or_insert(()).key().clone()
            });

        //let ptr = unsafe { NonNull::new_unchecked(Arc::into_raw(cell) as *mut Cell) };
        Atom2(cell)
    }
}

impl CachedStore {
    fn atom(&self, s: Cow<'_, str>) -> Atom2 {
        let hash = calc_hash(&s);
        let hash_key = HashKey { hash };
        let cell = self
            .cache
            .read(&hash_key, |cell, _| cell.clone())
            .unwrap_or_else(|| {
                let cell = Arc::new(Cell {
                    s: s.into_owned().into_boxed_str(),
                    hash,
                });
                self.cache.entry(cell).or_put(()).1.key().clone()
            });

        //let ptr = unsafe { NonNull::new_unchecked(Arc::into_raw(cell) as *mut Cell) };
        Atom2(cell)
    }
}

fn calc_hash(s: &str) -> u64 {
    let mut hasher = ahash::AHasher::default();
    s.hash(&mut hasher);
    hasher.finish()
}

impl Eq for Cell {}

impl PartialEq for Cell {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.s == other.s
    }
}

impl Hash for Cell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl Hash for Atom2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash);
    }
}

impl Display for Atom2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0.s, f)
    }
}

impl Debug for Atom2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0.s, f)
    }
}

impl bincode::Encode for Atom2 {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        self.as_str().encode(encoder)
    }
}

impl bincode::Decode for Atom2 {
    fn decode<D: bincode::de::Decoder>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        String::decode(decoder).map(Atom2::new)
    }
}

impl<'de> bincode::BorrowDecode<'de> for Atom2 {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        <&str>::borrow_decode(decoder).map(Atom2::new)
    }
}

impl serde::Serialize for Atom2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Atom2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <&str>::deserialize(deserializer).map(Atom2::new)
    }
}

impl Hash for HashKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl scc::Equivalent<Arc<Cell>> for HashKey {
    fn equivalent(&self, key: &Arc<Cell>) -> bool {
        key.hash == self.hash
    }
}

#[derive(Default)]
struct DirectHasher(u64);

impl Hasher for DirectHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unimplemented!("DirectHasher::write")
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}

#[test]
fn test_atom() {
    let foo = Atom2::new("foo");
    let bar = Atom2::new("bar");
    let bar2 = Atom2::new("bar");
    assert!(foo != bar);
    assert!(foo != "bar");
    assert!(bar == bar2);
    assert!(bar == "bar");

    // miri mem leak check
    GLOBAL_STORE.cache.clear();
}
