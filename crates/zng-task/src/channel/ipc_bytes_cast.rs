#![cfg_attr(not(ipc), allow(unused))]

use std::{fmt, io, iter::FusedIterator, marker::PhantomData, ops};

use crate::channel::{IpcBytes, IpcBytesIntoIter, IpcBytesMut};

/// Safe bytemuck casting wrapper for [`IpcBytesMut`].
///
/// Use [`IpcBytesMut::cast`] to cast.
pub struct IpcBytesMutCast<T: bytemuck::AnyBitPattern> {
    bytes: IpcBytesMut,
    _t: PhantomData<T>,
}
impl<T: bytemuck::AnyBitPattern> ops::Deref for IpcBytesMutCast<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice::<u8, T>(&self.bytes)
    }
}
impl<T: bytemuck::AnyBitPattern + bytemuck::NoUninit> ops::DerefMut for IpcBytesMutCast<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        bytemuck::cast_slice_mut::<u8, T>(&mut self.bytes)
    }
}
impl<T: bytemuck::AnyBitPattern> IpcBytesMutCast<T> {
    /// Convert back to [`IpcBytesMut`].
    pub fn into_inner(self) -> IpcBytesMut {
        self.bytes
    }
}
impl<T: bytemuck::AnyBitPattern> From<IpcBytesMutCast<T>> for IpcBytesMut {
    fn from(value: IpcBytesMutCast<T>) -> Self {
        value.bytes
    }
}
fn item_len_to_bytes<T: 'static>(len: usize) -> io::Result<usize> {
    match len.checked_mul(size_of::<T>()) {
        Some(l) => Ok(l),
        None => Err(io::Error::new(io::ErrorKind::FileTooLarge, "cannot map more than usize::MAX")),
    }
}
impl<T: bytemuck::AnyBitPattern + bytemuck::NoUninit> IpcBytesMutCast<T> {
    /// Allocate zeroed mutable memory.
    pub async fn new(len: usize) -> io::Result<Self> {
        IpcBytesMut::new(item_len_to_bytes::<T>(len)?).await.map(IpcBytesMut::cast)
    }

    /// Allocate zeroed mutable memory.
    pub fn new_blocking(len: usize) -> io::Result<Self> {
        IpcBytesMut::new_blocking(item_len_to_bytes::<T>(len)?).map(IpcBytesMut::cast)
    }

    /// Allocate zeroed mutable memory in a memory map.
    ///
    /// Note that [`new`] automatically selects the best memory storage for the given `len`, this
    /// function enforces the usage of a memory map, the slowest of the options.
    ///
    /// [`new`]: Self::new
    pub async fn new_memmap(len: usize) -> io::Result<Self> {
        IpcBytesMut::new_memmap(item_len_to_bytes::<T>(len)?).await.map(IpcBytesMut::cast)
    }

    /// Allocate zeroed mutable memory in a memory map.
    ///
    /// Note that [`new_blocking`] automatically selects the best memory storage for the given `len`, this
    /// function enforces the usage of a memory map, the slowest of the options.
    ///
    /// [`new_blocking`]: Self::new_blocking
    pub fn new_memmap_blocking(len: usize) -> io::Result<Self> {
        IpcBytesMut::new_memmap_blocking(item_len_to_bytes::<T>(len)?).map(IpcBytesMut::cast)
    }

    /// Uses `buf` or copies it to exclusive mutable memory.
    pub async fn from_vec(data: Vec<T>) -> io::Result<Self> {
        IpcBytesMut::from_vec(bytemuck::cast_vec(data)).await.map(IpcBytesMut::cast)
    }

    /// Uses `buf` or copies it to exclusive mutable memory.
    pub fn from_vec_blocking(data: Vec<T>) -> io::Result<Self> {
        IpcBytesMut::from_vec_blocking(bytemuck::cast_vec(data)).map(IpcBytesMut::cast)
    }

    /// Copy data from slice.
    pub fn from_slice_blocking(data: &[T]) -> io::Result<Self> {
        IpcBytesMut::from_slice_blocking(bytemuck::cast_slice(data)).map(IpcBytesMut::cast)
    }

    /// Reference the underlying raw bytes.
    pub fn as_bytes(&mut self) -> &mut IpcBytesMut {
        &mut self.bytes
    }
}
impl<T: bytemuck::AnyBitPattern + bytemuck::NoUninit> IpcBytesMutCast<T> {
    /// Convert to immutable shareable [`IpcBytesCast`].
    pub async fn finish(self) -> io::Result<IpcBytesCast<T>> {
        self.bytes.finish().await.map(IpcBytes::cast)
    }

    /// Convert to immutable shareable [`IpcBytesCast`].
    pub fn finish_blocking(self) -> io::Result<IpcBytesCast<T>> {
        self.bytes.finish_blocking().map(IpcBytes::cast)
    }
}

impl IpcBytesMut {
    /// Safe bytemuck casting wrapper.
    ///
    /// The wrapper will deref to `[T]` and can be converted back to `IpcBytesMust`.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast<T: bytemuck::AnyBitPattern>(self) -> IpcBytesMutCast<T> {
        let r = IpcBytesMutCast {
            bytes: self,
            _t: PhantomData,
        };
        let _assert = &r[..];
        r
    }

    /// Safe bytemuck cast to slice.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast_deref<T: bytemuck::AnyBitPattern>(&self) -> &[T] {
        bytemuck::cast_slice(self)
    }

    /// Safe bytemuck cast to mutable slice.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast_deref_mut<T: bytemuck::AnyBitPattern + bytemuck::NoUninit>(&mut self) -> &mut [T] {
        bytemuck::cast_slice_mut(self)
    }
}

/// Safe bytemuck casting wrapper for [`IpcBytes`].
///
/// Use [`IpcBytes::cast`] to cast.
pub struct IpcBytesCast<T: bytemuck::AnyBitPattern> {
    bytes: IpcBytes,
    _t: PhantomData<T>,
}
impl<T: bytemuck::AnyBitPattern> Default for IpcBytesCast<T> {
    fn default() -> Self {
        Self {
            bytes: Default::default(),
            _t: PhantomData,
        }
    }
}
impl<T: bytemuck::AnyBitPattern> ops::Deref for IpcBytesCast<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice::<u8, T>(&self.bytes)
    }
}
impl<T: bytemuck::AnyBitPattern> IpcBytesCast<T> {
    /// Convert back to [`IpcBytes`].
    pub fn into_inner(self) -> IpcBytes {
        self.bytes
    }
}
impl<T: bytemuck::AnyBitPattern> From<IpcBytesCast<T>> for IpcBytes {
    fn from(value: IpcBytesCast<T>) -> Self {
        value.bytes
    }
}
impl<T: bytemuck::AnyBitPattern> Clone for IpcBytesCast<T> {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            _t: PhantomData,
        }
    }
}
impl<T: bytemuck::AnyBitPattern> fmt::Debug for IpcBytesCast<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IpcBytesCast<{}>(<{} items>)", std::any::type_name::<T>(), self.len())
    }
}
impl<T: bytemuck::AnyBitPattern> serde::Serialize for IpcBytesCast<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bytes.serialize(serializer)
    }
}
impl<'de, T: bytemuck::AnyBitPattern> serde::Deserialize<'de> for IpcBytesCast<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = IpcBytes::deserialize(deserializer)?;
        Ok(bytes.cast())
    }
}
impl<T: bytemuck::AnyBitPattern> PartialEq for IpcBytesCast<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
impl<T: bytemuck::AnyBitPattern> Eq for IpcBytesCast<T> {}
impl<T: bytemuck::AnyBitPattern + bytemuck::NoUninit> IpcBytesCast<T> {
    /// Copy or move data from vector.
    pub async fn from_vec(data: Vec<T>) -> io::Result<Self> {
        IpcBytes::from_vec(bytemuck::cast_vec(data)).await.map(IpcBytes::cast)
    }

    /// Copy data from the iterator.
    ///
    /// This is most efficient if the [`size_hint`] indicates an exact length (min equals max), otherwise this
    /// will collect to an [`IpcBytesWriter`] that can reallocate multiple times as the buffer grows.
    ///
    /// Note that if the iterator gives an exact length that is the maximum taken, if it ends early the smaller length
    /// is used, if it continues after the given maximum it is clipped.
    ///
    /// [`size_hint`]: Iterator::size_hint
    /// [`IpcBytesWriter`]: crate::channel::IpcBytesWriter
    pub async fn from_iter(iter: impl Iterator<Item = T>) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let (min, max) = iter.size_hint();
            let l = size_of::<T>();
            let min = min * l;
            let max = max.map(|m| m * l);
            if let Some(max) = max {
                if max <= IpcBytes::INLINE_MAX {
                    return Self::from_vec(iter.collect()).await;
                } else if max == min {
                    let mut r = IpcBytesMut::new(max).await?;
                    let mut actual_len = 0;
                    for (i, f) in r.chunks_exact_mut(l).zip(iter) {
                        i.copy_from_slice(bytemuck::bytes_of(&f));
                        actual_len += 1;
                    }
                    r.truncate(actual_len * l);
                    return r.finish().await.map(IpcBytes::cast);
                }
            }

            let mut writer = IpcBytes::new_writer().await;
            for f in iter {
                use futures_lite::AsyncWriteExt as _;

                writer.write_all(bytemuck::bytes_of(&f)).await?;
            }
            writer.finish().await.map(IpcBytes::cast)
        }
        #[cfg(not(ipc))]
        {
            Self::from_vec(iter.collect()).await
        }
    }

    /// Copy or move data from vector.
    pub fn from_vec_blocking(data: Vec<T>) -> io::Result<Self> {
        IpcBytes::from_vec_blocking(bytemuck::cast_vec(data)).map(IpcBytes::cast)
    }

    /// Copy data from slice.
    pub fn from_slice_blocking(data: &[T]) -> io::Result<Self> {
        IpcBytes::from_slice_blocking(bytemuck::cast_slice(data)).map(IpcBytes::cast)
    }

    /// Copy data from the iterator.
    ///
    /// This is most efficient if the [`size_hint`] indicates an exact length (min equals max), otherwise this
    /// will collect to an [`IpcBytesWriterBlocking`] that can reallocate multiple times as the buffer grows.
    ///
    /// Note that if the iterator gives an exact length that is the maximum taken, if it ends early the smaller length
    /// is used, if it continues after the given maximum it is clipped.
    ///
    /// [`size_hint`]: Iterator::size_hint
    /// [`IpcBytesWriterBlocking`]: crate::channel::IpcBytesWriterBlocking
    pub fn from_iter_blocking(mut iter: impl Iterator<Item = T>) -> io::Result<Self> {
        #[cfg(ipc)]
        {
            let (min, max) = iter.size_hint();
            let l = size_of::<T>();
            let min = min * l;
            let max = max.map(|m| m * l);
            if let Some(max) = max {
                if max <= IpcBytes::INLINE_MAX {
                    return Self::from_vec_blocking(iter.collect());
                } else if max == min {
                    let mut r = IpcBytesMut::new_blocking(max)?;
                    let mut actual_len = 0;
                    for (i, f) in r.chunks_exact_mut(l).zip(&mut iter) {
                        i.copy_from_slice(bytemuck::bytes_of(&f));
                        actual_len += 1;
                    }
                    r.truncate(actual_len * l);
                    return r.finish_blocking().map(IpcBytes::cast);
                }
            }

            let mut writer = IpcBytes::new_writer_blocking();
            for f in iter {
                use std::io::Write as _;

                writer.write_all(bytemuck::bytes_of(&f))?;
            }
            writer.finish().map(IpcBytes::cast)
        }
        #[cfg(not(ipc))]
        {
            Self::from_vec_blocking(iter.collect())
        }
    }

    /// Reference the underlying raw bytes.
    pub fn as_bytes(&self) -> &IpcBytes {
        &self.bytes
    }
}

impl IpcBytes {
    /// Safe bytemuck casting wrapper.
    ///
    /// The wrapper will deref to `[T]` and can be converted back to `IpcBytes`.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast<T: bytemuck::AnyBitPattern>(self) -> IpcBytesCast<T> {
        let r = IpcBytesCast {
            bytes: self,
            _t: PhantomData,
        };
        let _assert = &r[..];
        r
    }

    /// Safe bytemuck cast to slice.
    ///
    /// # Panics
    ///
    /// Panics if cannot cast, se [bytemuck docs] for details.
    ///
    /// [bytemuck docs]: https://docs.rs/bytemuck/1.24.0/bytemuck/fn.try_cast_slice.html
    pub fn cast_deref<T: bytemuck::AnyBitPattern>(&self) -> &[T] {
        bytemuck::cast_slice(self)
    }
}

/// An [`IpcBytesCast`] iterator that holds a strong reference to it.
pub struct IpcBytesCastIntoIter<T: bytemuck::AnyBitPattern>(IpcBytesIntoIter, IpcBytesCast<T>);
impl<T: bytemuck::AnyBitPattern> IpcBytesCastIntoIter<T> {
    fn new(bytes: IpcBytesCast<T>) -> Self {
        Self(bytes.bytes.clone().into_iter(), bytes)
    }

    /// The source bytes.
    pub fn source(&self) -> &IpcBytesCast<T> {
        &self.1
    }

    /// Items not yet iterated.
    pub fn rest(&self) -> &[T] {
        bytemuck::cast_slice(self.0.rest())
    }
}
impl<T: bytemuck::AnyBitPattern> Iterator for IpcBytesCastIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let size = size_of::<T>();
        let r = *bytemuck::from_bytes(self.0.rest().get(..size)?);
        self.0.nth(size - 1);
        Some(r)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (mut min, mut max) = self.0.size_hint();
        min /= size_of::<T>();
        if let Some(max) = &mut max {
            *max /= size_of::<T>();
        }
        (min, max)
    }

    fn nth(&mut self, n: usize) -> Option<T> {
        let size = size_of::<T>();

        let byte_skip = n.checked_mul(size)?;
        let byte_end = byte_skip.checked_add(size)?;

        let bytes = self.0.rest().get(byte_skip..byte_end)?;
        let r = *bytemuck::from_bytes(bytes);

        self.0.nth(byte_end - 1);

        Some(r)
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.next_back()
    }
}
impl<T: bytemuck::AnyBitPattern> DoubleEndedIterator for IpcBytesCastIntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        let size = size_of::<T>();

        let len = self.0.rest().len();
        if len < size {
            return None;
        }

        let start = len - size;
        let bytes = &self.0.rest()[start..];
        let r = *bytemuck::from_bytes(bytes);

        self.0.nth_back(size - 1);

        Some(r)
    }

    fn nth_back(&mut self, n: usize) -> Option<T> {
        let size = size_of::<T>();

        let rev_byte_skip = n.checked_mul(size)?;
        let rev_byte_end = rev_byte_skip.checked_add(size)?;
        let len = self.0.rest().len();

        if len < rev_byte_end {
            return None;
        }

        let start = len - rev_byte_end;
        let end = len - rev_byte_skip;

        let bytes = &self.0.rest()[start..end];
        let r = *bytemuck::from_bytes(bytes);

        self.0.nth_back(rev_byte_end - 1);

        Some(r)
    }
}
impl<T: bytemuck::AnyBitPattern> FusedIterator for IpcBytesCastIntoIter<T> {}
impl<T: bytemuck::AnyBitPattern> Default for IpcBytesCastIntoIter<T> {
    fn default() -> Self {
        IpcBytes::empty().cast::<T>().into_iter()
    }
}
impl<T: bytemuck::AnyBitPattern> IntoIterator for IpcBytesCast<T> {
    type Item = T;

    type IntoIter = IpcBytesCastIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IpcBytesCastIntoIter::new(self)
    }
}
