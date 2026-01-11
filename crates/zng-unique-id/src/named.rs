use std::{
    fmt,
    hash::{BuildHasher, Hash},
};

use std::collections::hash_map;

use crate::{IdEntry, IdMap};

struct ConstDefaultHashBuilder;
impl BuildHasher for ConstDefaultHashBuilder {
    type Hasher = std::collections::hash_map::DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        std::collections::hash_map::DefaultHasher::default()
    }
}

type DefaultHashMap<K, V> = std::collections::HashMap<K, V, ConstDefaultHashBuilder>;

const fn default_hash_map_new<K, V>() -> DefaultHashMap<K, V> {
    DefaultHashMap::with_hasher(ConstDefaultHashBuilder)
}

#[doc(hidden)]
pub use zng_txt::Txt;

/// Bidirectional map between a `Txt` and a [`unique_id!`] generated id type.
struct NameIdMap<I> {
    name_to_id: DefaultHashMap<Txt, I>,
    id_to_name: IdMap<I, Txt>,
}
impl<I> NameIdMap<I>
where
    I: Copy + PartialEq + Eq + Hash + fmt::Debug,
{
    pub const fn new() -> Self {
        NameIdMap {
            name_to_id: default_hash_map_new(),
            id_to_name: IdMap::new(),
        }
    }

    pub fn set(&mut self, name: Txt, id: I) -> Result<(), IdNameError<I>> {
        if name.is_empty() {
            return Ok(());
        }

        match self.id_to_name.entry(id) {
            IdEntry::Occupied(e) => {
                if *e.get() == name {
                    Ok(())
                } else {
                    Err(IdNameError::AlreadyNamed(e.get().clone()))
                }
            }
            IdEntry::Vacant(e) => match self.name_to_id.entry(name.clone()) {
                hash_map::Entry::Occupied(ne) => Err(IdNameError::NameUsed(*ne.get())),
                hash_map::Entry::Vacant(ne) => {
                    e.insert(name);
                    ne.insert(id);
                    Ok(())
                }
            },
        }
    }

    pub fn get_id_or_insert(&mut self, name: Txt, new_unique: impl FnOnce() -> I) -> I {
        if name.is_empty() {
            return new_unique();
        }
        match self.name_to_id.entry(name.clone()) {
            hash_map::Entry::Occupied(e) => *e.get(),
            hash_map::Entry::Vacant(e) => {
                let id = new_unique();
                e.insert(id);
                self.id_to_name.insert(id, name);
                id
            }
        }
    }

    pub fn new_named(&mut self, name: Txt, new_unique: impl FnOnce() -> I) -> Result<I, IdNameError<I>> {
        if name.is_empty() {
            Ok(new_unique())
        } else {
            match self.name_to_id.entry(name.clone()) {
                hash_map::Entry::Occupied(e) => Err(IdNameError::NameUsed(*e.get())),
                hash_map::Entry::Vacant(e) => {
                    let id = new_unique();
                    e.insert(id);
                    self.id_to_name.insert(id, name);
                    Ok(id)
                }
            }
        }
    }

    pub fn get_name(&self, id: I) -> Txt {
        self.id_to_name.get(&id).cloned().unwrap_or_default()
    }
}

/// Error when trying to associate give a name with an existing id.
#[derive(Clone, Debug)]
pub enum IdNameError<I: Clone + Copy + fmt::Debug> {
    /// The id is already named, id names are permanent.
    ///
    /// The associated value if the id name.
    AlreadyNamed(Txt),
    /// The name is already used for another id, names must be unique.
    ///
    /// The associated value if the named id.
    NameUsed(I),
}
impl<I: Clone + Copy + fmt::Debug> fmt::Display for IdNameError<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdNameError::AlreadyNamed(name) => write!(f, "cannot name the id, it is already called `{name:?}`"),
            IdNameError::NameUsed(id) => write!(f, "cannot name the id, it is already the name of {id:#?}"),
        }
    }
}
impl<I: Clone + Copy + fmt::Debug> std::error::Error for IdNameError<I> {}

#[doc(hidden)]
pub struct UniqueIdNameStore<I>(parking_lot::RwLock<NameIdMap<I>>);
impl<I> UniqueIdNameStore<I>
where
    I: Copy + PartialEq + Eq + Hash + fmt::Debug,
{
    pub const fn new() -> Self {
        Self(parking_lot::const_rwlock(NameIdMap::new()))
    }

    pub fn named(&self, name: impl Into<Txt>, new_unique: impl FnOnce() -> I) -> I {
        self.0.write().get_id_or_insert(name.into(), new_unique)
    }

    pub fn named_new(&self, name: impl Into<Txt>, new_unique: impl FnOnce() -> I) -> Result<I, IdNameError<I>> {
        self.0.write().new_named(name.into(), new_unique)
    }

    pub fn name(&self, id: I) -> Txt {
        self.0.read().get_name(id)
    }

    pub fn set_name(&self, name: impl Into<Txt>, id: I) -> Result<(), IdNameError<I>> {
        self.0.write().set(name.into(), id)
    }
}
impl<I> Default for UniqueIdNameStore<I>
where
    I: Copy + PartialEq + Eq + Hash + fmt::Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Extend an unique ID type to have an optional attached name string, also implements.
#[macro_export]
macro_rules! impl_unique_id_name {
    ($UniqueId:ident) => {
        $crate::paste! {
            $crate::hot_static! {
                static [<$UniqueId:upper _ID_NAMES>]: $crate::UniqueIdNameStore<$UniqueId> = $crate::UniqueIdNameStore::new();
            }
        }

        impl $UniqueId {
            fn names_store() -> &'static $crate::UniqueIdNameStore<Self> {
                $crate::paste! {
                    $crate::hot_static_ref! {
                        [<$UniqueId:upper _ID_NAMES>]
                    }
                }
            }

            /// Get or generate an ID with associated name.
            ///
            /// If the `name` is already associated with an ID, returns it.
            /// If the `name` is new, generates a new ID and associated it with the name.
            /// If `name` is an empty string just returns a new ID.
            pub fn named(name: impl Into<$crate::Txt>) -> Self {
                Self::names_store().named(name, Self::new_unique)
            }

            /// Calls [`named`] in a debug build and [`new_unique`] in a release build.
            ///
            /// [`named`]: Self::named
            /// [`new_unique`]: Self::new_unique
            pub fn debug_named(name: impl Into<$crate::Txt>) -> Self {
                #[cfg(debug_assertions)]
                return Self::named(name);

                #[cfg(not(debug_assertions))]
                {
                    let _ = name;
                    Self::new_unique()
                }
            }

            /// Generate a new ID with associated name.
            ///
            /// If the `name` is already associated with an ID, returns the `NameUsed` error.
            /// If the `name` is an empty string just returns a new ID.
            pub fn named_new(name: impl Into<$crate::Txt>) -> std::result::Result<Self, $crate::IdNameError<Self>> {
                Self::names_store().named_new(name.into(), Self::new_unique)
            }

            /// Returns the name associated with the ID or `""`.
            pub fn name(self) -> $crate::Txt {
                Self::names_store().name(self)
            }

            /// Associate a `name` with the ID, if it is not named.
            ///
            /// If the `name` is already associated with a different ID, returns the `NameUsed` error.
            /// If the ID is already named, with a name different from `name`, returns the `AlreadyNamed` error.
            /// If the `name` is an empty string or already is the name of the ID, does nothing.
            pub fn set_name(self, name: impl Into<$crate::Txt>) -> std::result::Result<(), $crate::IdNameError<Self>> {
                Self::names_store().set_name(name.into(), self)
            }
        }
    };
}

/// Implement debug and display for an unique ID type that also implements name.
#[macro_export]
macro_rules! impl_unique_id_fmt {
    ($UniqueId:ident) => {
        impl std::fmt::Debug for $UniqueId {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let name = self.name();
                if f.alternate() {
                    f.debug_struct(stringify!($UniqueId))
                        .field("id", &self.get())
                        .field("sequential", &self.sequential())
                        .field("name", &name)
                        .finish()
                } else if !name.is_empty() {
                    write!(f, r#"{}("{name}")"#, stringify!($UniqueId))
                } else {
                    write!(f, "{}({})", stringify!($UniqueId), self.sequential())
                }
            }
        }
        impl std::fmt::Display for $UniqueId {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let name = self.name();
                if !name.is_empty() {
                    write!(f, "{name}")
                } else if f.alternate() {
                    write!(f, "#{}", self.sequential())
                } else {
                    write!(f, "{}({})", stringify!($UniqueId), self.sequential())
                }
            }
        }
    };
}
