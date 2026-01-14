//!

use core::cmp;
use core::error;
use core::fmt;

type Result<T> = core::result::Result<T, Error>;

/// Kinds of errors that can occur while reading or writing with a cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An integer overflow occurred.
    Overflow,
    /// An unexpected end of file was encountered.
    UnexpectedEof {
        /// Number of bytes
        expected: usize,
        /// Number of bytes remaining
        remaining: usize,
    },
    /// The buffer is too small to complete the operation.
    BufferTooSmall {
        /// Number of bytes needed.
        expected: usize,
        /// Number of bytes remaining in the buffer.
        remaining: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Overflow => write!(f, "Integer overflow occurred"),
            Error::UnexpectedEof {
                expected,
                remaining,
            } => write!(
                f,
                "Unexpected end of file: needed {} bytes but only {} remaining",
                expected, remaining
            ),
            Error::BufferTooSmall {
                expected,
                remaining,
            } => write!(
                f,
                "Buffer too small: needed {} bytes but only {} remaining",
                expected, remaining
            ),
        }
    }
}

impl error::Error for Error {}

/// A cursor for reading from a byte slice.
#[derive(Clone)]
pub struct ReadCursor<'a> {
    inner: &'a [u8],
    pos: usize,
}

impl<'a> ReadCursor<'a> {
    /// Creates a new `ReadCursor` from the given byte slice.
    #[inline]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self {
            inner: bytes,
            pos: 0,
        }
    }

    /// Returns the total length of the inner byte slice.
    #[inline]
    #[track_caller]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the number of bytes remaining to be read.
    #[inline]
    #[track_caller]
    pub const fn remaining(&self) -> usize {
        self.inner.len() - self.pos
    }

    /// Returns `true` if there are no bytes left to read.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Returns a slice of the remaining unread bytes.
    #[inline]
    #[track_caller]
    pub fn remaining_slice(&self) -> &'a [u8] {
        let idx = cmp::min(self.pos, self.inner.len());
        &self.inner[idx..]
    }

    /// Returns the entire inner byte slice.
    #[inline]
    pub const fn inner(&self) -> &[u8] {
        self.inner
    }

    /// Returns the current position of the cursor.
    #[inline]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Advances the cursor by `n` bytes and returns a slice of the taken bytes.
    #[inline]
    #[track_caller]
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let start = self.pos;
        let end = start.checked_add(n).ok_or(Error::Overflow)?;
        let bytes = self.inner.get(start..end).ok_or(Error::UnexpectedEof {
            expected: n,
            remaining: self.remaining(),
        })?;

        self.pos = end;
        Ok(bytes)
    }

    /// Advances the cursor by `len` bytes.
    #[inline]
    #[track_caller]
    pub fn advance(&mut self, len: usize) -> Result<()> {
        self.take(len).map(|_| ())
    }

    /// Reads an array of `N` bytes from the cursor.
    #[inline]
    #[track_caller]
    pub fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.take(N)?;
        Ok(bytes.try_into().expect("N-elements array"))
    }

    /// Reads a single byte from the cursor.
    #[inline]
    #[track_caller]
    pub fn read_u8(&mut self) -> Result<u8> {
        let bytes = self.read_array::<1>()?;
        Ok(bytes[0])
    }

    /// Reads a big-endian 16-bit unsigned integer from the cursor.
    #[inline]
    #[track_caller]
    pub fn read_u16_be(&mut self) -> Result<u16> {
        let bytes = self.read_array::<2>()?;
        Ok(u16::from_be_bytes(bytes))
    }

    /// Reads a big-endian 16-bit signed integer from the cursor.
    #[inline]
    #[track_caller]
    pub fn read_i16_be(&mut self) -> Result<i16> {
        let bytes = self.read_array::<2>()?;
        Ok(i16::from_be_bytes(bytes))
    }

    /// Reads a big-endian 32-bit unsigned integer from the cursor.
    #[inline]
    #[track_caller]
    pub fn read_u32_be(&mut self) -> Result<u32> {
        let bytes = self.read_array::<4>()?;
        Ok(u32::from_be_bytes(bytes))
    }

    /// Reads a big-endian 32-bit signed integer from the cursor.
    #[inline]
    #[track_caller]
    pub fn read_i32_be(&mut self) -> Result<i32> {
        let bytes = self.read_array::<4>()?;
        Ok(i32::from_be_bytes(bytes))
    }

    /// Reads a big-endian 64-bit unsigned integer from the cursor.
    #[inline]
    #[track_caller]
    pub fn read_u64_be(&mut self) -> Result<u64> {
        let bytes = self.read_array::<8>()?;
        Ok(u64::from_be_bytes(bytes))
    }
}

/// A cursor for writing to a byte slice.
pub struct WriteCursor<'a> {
    inner: &'a mut [u8],
    pos: usize,
}

impl<'a> WriteCursor<'a> {
    /// Creates a new `WriteCursor` from the given mutable byte slice.
    #[inline]
    pub const fn new(bytes: &'a mut [u8]) -> Self {
        Self {
            inner: bytes,
            pos: 0,
        }
    }

    /// Returns the total length of the inner byte slice.
    #[inline]
    #[track_caller]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns the number of bytes remaining to be written.
    #[inline]
    #[track_caller]
    pub const fn remaining(&self) -> usize {
        self.inner.len() - self.pos
    }

    /// Returns `true` if there are no bytes left to write.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// Returns the entire inner byte slice.
    #[inline]
    pub const fn inner(&self) -> &[u8] {
        self.inner
    }

    /// Returns the entire inner mutable byte slice.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut [u8] {
        self.inner
    }

    /// Returns the current position of the cursor.
    #[inline]
    pub const fn position(&self) -> usize {
        self.pos
    }

    /// Advances the cursor by `n` bytes and returns a mutable slice of the taken bytes.
    #[inline]
    #[track_caller]
    pub fn take_mut(&mut self, n: usize) -> Result<&mut [u8]> {
        let start = self.pos;
        let end = start.checked_add(n).ok_or(Error::Overflow)?;
        if end > self.inner.len() {
            return Err(Error::BufferTooSmall {
                expected: n,
                remaining: self.remaining(),
            });
        }

        self.pos = end;
        Ok(&mut self.inner[start..end])
    }

    /// Reserves `n` bytes in the cursor and fills them with zeros. Returns the starting position.
    #[inline]
    #[track_caller]
    pub fn reserve_zeros(&mut self, n: usize) -> Result<usize> {
        let at = self.pos;
        let buf = self.take_mut(n)?;
        buf.fill(0);
        Ok(at)
    }

    /// Writes a slice of bytes to the cursor.
    #[inline]
    #[track_caller]
    pub fn write_slice(&mut self, bytes: &[u8]) -> Result<()> {
        let dst = self.take_mut(bytes.len())?;
        dst.copy_from_slice(bytes);
        Ok(())
    }

    /// Writes an array of `N` bytes to the cursor.
    #[inline]
    #[track_caller]
    pub fn write_array<const N: usize>(&mut self, bytes: &[u8; N]) -> Result<()> {
        let dst = self.take_mut(N)?;
        dst.copy_from_slice(bytes);
        Ok(())
    }

    /// Writes a single byte to the cursor.
    #[inline]
    #[track_caller]
    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        self.write_array(&value.to_be_bytes())?;
        Ok(())
    }

    /// Writes a big-endian 16-bit unsigned integer to the cursor.
    #[inline]
    #[track_caller]
    pub fn write_u16_be(&mut self, value: u16) -> Result<()> {
        self.write_array(&value.to_be_bytes())?;
        Ok(())
    }

    /// Writes a big-endian 32-bit unsigned integer to the cursor.
    #[inline]
    #[track_caller]
    pub fn write_u32_be(&mut self, value: u32) -> Result<()> {
        self.write_array(&value.to_be_bytes())?;
        Ok(())
    }

    /// Writes a big-endian 32-bit signed integer to the cursor.
    #[inline]
    #[track_caller]
    pub fn write_i32_be(&mut self, value: i32) -> Result<()> {
        self.write_array(&value.to_be_bytes())?;
        Ok(())
    }

    /// Writes a big-endian 64-bit unsigned integer to the cursor.
    #[inline]
    #[track_caller]
    pub fn write_u64_be(&mut self, value: u64) -> Result<()> {
        self.write_array(&value.to_be_bytes())?;
        Ok(())
    }
}
