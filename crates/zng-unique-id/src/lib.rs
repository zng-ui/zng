#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Macros for generating unique ID types.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{
    hash::{BuildHasher, Hash, Hasher},
    num::{NonZeroU32, NonZeroU64},
    ops,
    sync::atomic::{AtomicU32, Ordering},
};

use rayon::iter::{FromParallelIterator, IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator};

#[doc(hidden)]
#[cfg(target_has_atomic = "64")]
pub use std::sync::atomic::AtomicU64;

#[doc(hidden)]
#[cfg(not(target_has_atomic = "64"))]
pub struct AtomicU64(parking_lot::Mutex<u64>);
#[cfg(not(target_has_atomic = "64"))]
impl AtomicU64 {
    pub const fn new(u: u64) -> Self {
        Self(parking_lot::Mutex::new(u))
    }

    fn fetch_add(&self, u: u64, _: Ordering) -> u64 {
        let mut a = self.0.lock();
        let r = *a;
        *a += u;
        r
    }
}

#[cfg(feature = "named")]
mod named;

#[doc(hidden)]
pub mod hot_reload;

pub use hot_reload::lazy_static_init;

#[cfg(feature = "named")]
pub use named::*;

#[doc(hidden)]
pub use pastey::paste;

/// Declare a new unique id type that is backed by a `NonZeroU32`.
#[macro_export]
macro_rules! unique_id_32 {
    ($(#[$attrs:meta])* $vis:vis struct $Type:ident $(< $T:ident $(:($($bounds:tt)+))? >)? $(: $ParentId:path)? ;) => {
       $crate::unique_id! {
            request {
                $(#[$attrs])*
                ///
                /// # Memory
                ///
                /// The internal number is a [`NonZeroU32`], that means that
                #[doc=concat!("`Option<", stringify!($Type), ">`")]
                /// and
                #[doc=concat!("`", stringify!($Type), "`")]
                /// are the same size as `u32`.
                ///
                /// # As Hash
                ///
                /// The generated internal number has good statistical distribution and can be used as its own hash,
                /// although it is not cryptographically safe, as it is simply a sequential counter scrambled using a modified
                /// `splitmix64`.
                ///
                /// [`NonZeroU32`]: std::num::NonZeroU32
                ///
                /// # Static
                ///
                /// The unique ID cannot be generated at compile time, but you can use the `static_id!` macro to declare
                /// a lazy static that instantiates the ID.
                ///
                /// # Exhaustion Handling
                ///
                /// If more IDs are generated them `u32::MAX` an error is logged, the internal counter is reset and ids are reused.
                $vis struct $Type $(< $T $(:($($bounds)+))? >)? $(: $ParentId)? ;
            }
            non_zero {
                std::num::NonZeroU32
            }
            atomic {
                std::sync::atomic::AtomicU32
            }
            next_id {
                $crate::next_id32
            }
            literal {
                u32
            }
            to_hash {
                $crate::un_hash32
            }
            to_sequential {
                $crate::un_hash32
            }
       }
    }
}

/// Declare a new unique id type that is backed by a `NonZeroU64`.
#[macro_export]
macro_rules! unique_id_64 {
    ($(#[$attrs:meta])* $vis:vis struct $Type:ident $(< $T:ident $(:($($bounds:tt)+))? >)? $(: $ParentId:path)? ;) => {
        $crate::unique_id! {
            request {
                $(#[$attrs])*
                ///
                /// # Memory
                ///
                /// The internal number is a [`NonZeroU64`], that means that
                #[doc=concat!("`Option<", stringify!($Type), ">`")]
                /// and
                #[doc=concat!("`", stringify!($Type), "`")]
                /// are the same size as `u64`.
                ///
                /// # As Hash
                ///
                /// The generated internal number has good statistical distribution and can be used as its own hash,
                /// although it is not cryptographically safe, as it is simply a sequential counter scrambled using `splitmix64`.
                ///
                /// [`NonZeroU64`]: std::num::NonZeroU64
                ///
                /// # Static
                ///
                /// The unique ID cannot be generated at compile time, but you can use the `static_id!` macro to declare
                /// a lazy static that instantiates the ID.
                ///
                /// # Exhaustion Handling
                ///
                /// If more IDs are generated them `u64::MAX` an error is logged, the internal counter is reset and ids are reused.
                $vis struct $Type $(< $T $(:($($bounds)+))? >)? $(: $ParentId)? ;
            }
            non_zero {
                std::num::NonZeroU64
            }
            atomic {
                $crate::AtomicU64
            }
            next_id {
                $crate::next_id64
            }
            literal {
                u64
            }
            to_hash {
                $crate::splitmix64
            }
            to_sequential {
                $crate::un_splitmix64
            }
        }
    };
}

/// Implement [`bytemuck`] trait for the unique ID.
///
/// [`bytemuck`]: https://docs.rs/bytemuck/
#[macro_export]
macro_rules! impl_unique_id_bytemuck {
    ($Type:ident $(< $T:ident $(:($($bounds:tt)+))? >)?) => {
        // SAFETY: $Type a transparent wrapper on a std non-zero integer.
        unsafe impl$(<$T $(: $($bounds)+)?>)? bytemuck::NoUninit for $Type $(<$T>)? { }
        unsafe impl$(<$T $(: $($bounds)+)?>)? bytemuck::ZeroableInOption for $Type $(<$T>)? { }
        unsafe impl$(<$T $(: $($bounds)+)?>)? bytemuck::PodInOption for $Type $(<$T>)? { }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! unique_id {
    (
        request {
            $(#[$attrs:meta])* $vis:vis struct $Type:ident $(< $T:ident $(:($($bounds:tt)+))? >)? $(: $ParentId:path)? ;
        }
        non_zero {
            $non_zero:path
        }
        atomic {
            $atomic:path
        }
        next_id {
            $next_id:path
        }
        literal {
            $lit:ident
        }
        to_hash {
            $to_hash:path
        }
        to_sequential {
            $to_sequential:path
        }
    ) => {

        $(#[$attrs])*
        #[repr(transparent)]
        $vis struct $Type $(<$T $(: $($bounds)+)?>)? ($non_zero $(, std::marker::PhantomData<$T>)?);

        impl$(<$T $(: $($bounds)+)?>)? Clone for $Type $(<$T>)? {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl$(<$T $(: $($bounds)+)?>)? Copy for $Type $(<$T>)? {
        }
        impl$(<$T $(: $($bounds)+)?>)? PartialEq for $Type $(<$T>)? {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }
        impl$(<$T $(: $($bounds)+)?>)? Eq for $Type $(<$T>)? {
        }
        impl$(<$T $(: $($bounds)+)?>)? std::hash::Hash for $Type $(<$T>)? {
            fn hash<H>(&self, state: &mut H)
            where
                H: std::hash::Hasher
            {
                std::hash::Hash::hash(&self.0, state)
            }
        }
        impl$(<$T $(: $($bounds)+)?>)? $crate::UniqueId for $Type $(<$T>)? {
            fn new_unique() -> Self {
                Self::new_unique()
            }
        }

        #[allow(dead_code)]
        impl$(<$T $(: $($bounds)+)?>)? $Type $(<$T>)? {
            $crate::unique_id! {
                new_unique {
                    $($ParentId, )? $(<$T>)?
                }
                atomic {
                    $atomic
                }
                next_id {
                    $next_id
                }
            }

            /// Retrieve the underlying value.
            pub fn get(self) -> $lit {
                self.0.get()
            }

            /// Un-scramble the underlying value to get the original sequential count number.
            ///
            /// If two IDs, `id0` and `id1` where generated by the same thread then `id0.sequential() < id1.sequential()`.
            pub fn sequential(self) -> $lit {
                $to_sequential(self.0.get())
            }

            /// Creates an ID from a raw value.
            ///
            /// The value must not be zero, panics if it is, the value must have been provided by [`get`] otherwise
            /// the ID will not be unique.
            ///
            /// [`get`]: Self::get
            pub fn from_raw(raw: $lit) -> Self {
                use $non_zero as __non_zero;

                Self(__non_zero::new(raw).unwrap() $(, std::marker::PhantomData::<$T>)?)
            }

            /// Creates an ID from a [`sequential`] number.
            ///
            /// # Safety
            ///
            /// The value must not be zero, panics if it is, the value must have been provided by [`sequential`] otherwise
            /// the ID will not be unique.
            ///
            /// [`sequential`]: Self::sequential
            pub fn from_sequential(num: $lit) -> Self {
                use $non_zero as __non_zero;

                Self(__non_zero::new($to_hash(num)).unwrap() $(, std::marker::PhantomData::<$T>)?)
            }
        }
    };

    (
        new_unique {
            $ParentId:path, $(<$T:ident>)?
        }
        atomic {
            $atomic:path
        }
        next_id {
            $next_id:path
        }
    ) => {
        /// Generates a new unique ID.
        pub fn new_unique() -> Self {
            use $ParentId as __parent;
            let id = __parent $(::<$T>)? ::new_unique().get();
            Self::from_raw(id)
        }
    };

    (
        new_unique {
            $(<$T:ident>)?
        }
        atomic {
            $atomic:path
        }
        next_id {
            $next_id:path
        }
    ) => {
        /// Generates a new unique ID.
        pub fn new_unique() -> Self {
            use $atomic as __atomic;

            $crate::hot_static! {
                static NEXT: __atomic = __atomic::new(1);
            }
            let __ref = $crate::hot_static_ref!(NEXT);
            Self($next_id(__ref) $(, std::marker::PhantomData::<$T>)?)
        }
    };
}

#[doc(hidden)]
pub fn next_id32(next: &'static AtomicU32) -> NonZeroU32 {
    loop {
        // the sequential next id is already in the variable.
        let id = next.fetch_add(1, Ordering::Relaxed);

        if id == 0 {
            tracing::error!("id factory reached `u32::MAX`, will start reusing");
        } else {
            let id = hash32(id);
            if let Some(id) = NonZeroU32::new(id) {
                return id;
            }
        }
    }
}
#[doc(hidden)]
pub fn next_id64(next: &'static AtomicU64) -> NonZeroU64 {
    loop {
        // the sequential next id is already in the variable.
        let id = next.fetch_add(1, Ordering::Relaxed);

        if id == 0 {
            tracing::error!("id factory reached `u64::MAX`, will start reusing");
        } else {
            // remove the sequential clustering.
            let id = splitmix64(id);
            if let Some(id) = NonZeroU64::new(id) {
                return id;
            }
        }
    }
}

#[doc(hidden)]
pub fn hash32(n: u32) -> u32 {
    use std::num::Wrapping as W;

    let mut z = W(n);
    z = ((z >> 16) ^ z) * W(0x45d9f3b);
    z = ((z >> 16) ^ z) * W(0x45d9f3b);
    z = (z >> 16) ^ z;
    z.0
}
#[doc(hidden)]
pub fn un_hash32(z: u32) -> u32 {
    use std::num::Wrapping as W;

    let mut n = W(z);
    n = ((n >> 16) ^ n) * W(0x119de1f3);
    n = ((n >> 16) ^ n) * W(0x119de1f3);
    n = (n >> 16) ^ n;
    n.0
}

#[doc(hidden)]
pub fn splitmix64(n: u64) -> u64 {
    use std::num::Wrapping as W;

    let mut z = W(n);
    z = (z ^ (z >> 30)) * W(0xBF58476D1CE4E5B9u64);
    z = (z ^ (z >> 27)) * W(0x94D049BB133111EBu64);
    z = z ^ (z >> 31);
    z.0
}
#[doc(hidden)]
pub fn un_splitmix64(z: u64) -> u64 {
    use std::num::Wrapping as W;

    let mut n = W(z);
    n = (n ^ (n >> 31) ^ (n >> 62)) * W(0x319642b2d24d8ec3u64);
    n = (n ^ (n >> 27) ^ (n >> 54)) * W(0x96de1b173f119089u64);
    n = n ^ (n >> 30) ^ (n >> 60);
    n.0
}

/// Map specialized for unique IDs that are already a randomized hash.
#[derive(Clone, Debug)]
pub struct IdMap<K, V>(hashbrown::HashMap<K, V, BuildIdHasher>);
impl<K, V> IdMap<K, V> {
    /// New `const` default.
    pub const fn new() -> Self {
        Self(hashbrown::HashMap::with_hasher(BuildIdHasher))
    }
}
impl<K, V> Default for IdMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K, V> ops::Deref for IdMap<K, V> {
    type Target = hashbrown::HashMap<K, V, BuildIdHasher>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<K, V> ops::DerefMut for IdMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<K, V> IntoIterator for IdMap<K, V> {
    type Item = (K, V);

    type IntoIter = hashbrown::hash_map::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a, K, V> IntoIterator for &'a IdMap<K, V> {
    type Item = (&'a K, &'a V);

    type IntoIter = hashbrown::hash_map::Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<'a, K, V> IntoIterator for &'a mut IdMap<K, V> {
    type Item = (&'a K, &'a mut V);

    type IntoIter = hashbrown::hash_map::IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}
impl<K: Send, V: Send> IntoParallelIterator for IdMap<K, V> {
    type Iter = hashbrown::hash_map::rayon::IntoParIter<K, V>;

    type Item = (K, V);

    fn into_par_iter(self) -> Self::Iter {
        self.0.into_par_iter()
    }
}
impl<'a, K: Sync, V: Sync> IntoParallelIterator for &'a IdMap<K, V> {
    type Iter = hashbrown::hash_map::rayon::ParIter<'a, K, V>;

    type Item = (&'a K, &'a V);

    fn into_par_iter(self) -> Self::Iter {
        self.0.par_iter()
    }
}
impl<'a, K: Sync, V: Send> IntoParallelIterator for &'a mut IdMap<K, V> {
    type Iter = hashbrown::hash_map::rayon::ParIterMut<'a, K, V>;

    type Item = (&'a K, &'a mut V);

    fn into_par_iter(self) -> Self::Iter {
        self.0.par_iter_mut()
    }
}
impl<K: Eq + Hash, V> FromIterator<(K, V)> for IdMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        Self(FromIterator::from_iter(iter))
    }
}
impl<K: Eq + Hash + Send, V: Send> FromParallelIterator<(K, V)> for IdMap<K, V> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = (K, V)>,
    {
        Self(FromParallelIterator::from_par_iter(par_iter))
    }
}

/// Set specialized for unique IDs that are already a randomized hash.
#[derive(Clone, Debug)]
pub struct IdSet<K>(hashbrown::HashSet<K, BuildIdHasher>);
impl<K> IdSet<K> {
    /// New `const` default.
    pub const fn new() -> Self {
        Self(hashbrown::HashSet::with_hasher(BuildIdHasher))
    }
}
impl<K> Default for IdSet<K> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K> ops::Deref for IdSet<K> {
    type Target = hashbrown::HashSet<K, BuildIdHasher>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<K> ops::DerefMut for IdSet<K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<K> IntoIterator for IdSet<K> {
    type Item = K;

    type IntoIter = hashbrown::hash_set::IntoIter<K>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a, K> IntoIterator for &'a IdSet<K> {
    type Item = &'a K;

    type IntoIter = hashbrown::hash_set::Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
impl<K: Send> IntoParallelIterator for IdSet<K> {
    type Iter = hashbrown::hash_set::rayon::IntoParIter<K>;

    type Item = K;

    fn into_par_iter(self) -> Self::Iter {
        self.0.into_par_iter()
    }
}
impl<'a, K: Sync> IntoParallelIterator for &'a IdSet<K> {
    type Iter = hashbrown::hash_set::rayon::ParIter<'a, K>;

    type Item = &'a K;

    fn into_par_iter(self) -> Self::Iter {
        self.0.par_iter()
    }
}
impl<K: Eq + Hash> FromIterator<K> for IdSet<K> {
    fn from_iter<T: IntoIterator<Item = K>>(iter: T) -> Self {
        Self(FromIterator::from_iter(iter))
    }
}
impl<K: Eq + Hash + Send> FromParallelIterator<K> for IdSet<K> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = K>,
    {
        Self(FromParallelIterator::from_par_iter(par_iter))
    }
}
impl<K: Eq + Hash> PartialEq for IdSet<K> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<K: Eq + Hash> Eq for IdSet<K> {}

/// Entry in [`IdMap`].
pub type IdEntry<'a, K, V> = hashbrown::hash_map::Entry<'a, K, V, BuildIdHasher>;

/// Occupied entry in an [`IdEntry`].
pub type IdOccupiedEntry<'a, K, V> = hashbrown::hash_map::OccupiedEntry<'a, K, V, BuildIdHasher>;

/// Vacant entry in an [`IdEntry`].
pub type IdVacantEntry<'a, K, V> = hashbrown::hash_map::VacantEntry<'a, K, V, BuildIdHasher>;

/// Build [`IdHasher`].
#[derive(Default, Clone, Debug, Copy)]
pub struct BuildIdHasher;
impl BuildHasher for BuildIdHasher {
    type Hasher = IdHasher;

    fn build_hasher(&self) -> Self::Hasher {
        IdHasher::default()
    }
}

/// No-op hasher.
///
/// This hasher supports only `write_u32` and `write_u64`, other methods panic.
///
/// This hasher does nothing, it uses the `u32` or `u64` value directly as a hash.
#[derive(Default)]
pub struct IdHasher(u64);
impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unimplemented!("`only `write_u32` and `write_u64` are supported");
    }

    fn write_u32(&mut self, id: u32) {
        self.0 = id as u64;
    }

    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// Trait implemented for all generated unique ID types.
pub trait UniqueId: Clone + Copy + PartialEq + Eq + Hash {
    /// New unique ID.
    fn new_unique() -> Self;
}

/// Declares a static unique ID that is lazy inited.
///
/// Dereferencing this static generates the ID and caches it.
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zng_unique_id::*;
/// #
/// # unique_id_32! {
/// #     pub struct StateId<T: (std::any::Any)>;
/// # }
/// #
/// static_id! {
///     /// Metadata foo ID.
///     pub static ref FOO_ID: StateId<bool>;
/// }
/// ```
#[macro_export]
macro_rules! static_id {
    ($(
        $(#[$attr:meta])*
        $vis:vis static ref $IDENT:ident: $IdTy:ty;
    )+) => {
        $(
            $crate::lazy_static! {
                $(#[$attr])*
                $vis static ref $IDENT: $IdTy = <$IdTy as $crate::UniqueId>::new_unique();
            }
        )+
    };
}
