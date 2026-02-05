//! BMFF box codec traits and functions.
//!
//! This module defines traits for encoding and decoding BMFF boxes,
//! as well as functions for reading and writing boxes to and from byte slices.
//!
//! # Traits
//!
//! The codec system is built around three traits:
//!
//! - [`BoxCodec`]: Identifies a box by its type (required for all boxes)
//! - [`BoxDecode`]: Parses a box from a byte slice (for reading)
//! - [`BoxEncode`]: Serializes a box to a byte slice (for writing)
//!
//! # Reading Boxes
//!
//! Use [`read_box`] to parse a complete box (header + payload) from bytes:
//!
//! ```
//! use mp4_bmff::codec::read_box;
//! use mp4_bmff::boxes::bmff::FtypBoxView;
//!
//! let data = [
//!     // Box header: size=20, type="ftyp"
//!     0x00, 0x00, 0x00, 0x14,
//!     b'f', b't', b'y', b'p',
//!     // Payload: major_brand="isom", minor_version=0, brands=["isom"]
//!     b'i', b's', b'o', b'm',
//!     0x00, 0x00, 0x00, 0x00,
//!     b'i', b's', b'o', b'm',
//! ];
//!
//! let ftyp: FtypBoxView = read_box(&data).unwrap();
//! assert_eq!(ftyp.major_brand.as_bytes(), b"isom");
//! ```
//!
//! # Writing Boxes
//!
//! Use [`write_box`] to serialize a box (header + payload) to bytes:
//!
//! ```
//! use mp4_bmff::codec::write_box;
//! use mp4_bmff::boxes::bmff::{MfroBox, MfroFlags};
//!
//! let mfro = MfroBox {
//!     version: 0,
//!     flags: MfroFlags::empty(),
//!     size: 1234,
//! };
//! let mut buffer = [0u8; 20];
//! let written = write_box(&mut buffer, &mfro).unwrap();
//!
//! assert_eq!(written, 16); // 8 (header) + 4 (version/flags) + 4 (size)
//! ```

use crate::base::header::BoxHeader;
use crate::base::header::BoxType;
use crate::base::rawbox::RawBox;

use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

#[cfg(feature = "alloc")]
use crate::lib::Vec;

/// Trait identifying a BMFF box by its type.
///
/// All box types must implement this trait to provide their box type
/// identifier, which is used when constructing box headers during encoding.
///
/// # Implementation
///
/// The box type is typically a 4-byte FourCC code (e.g., "ftyp", "moov"),
/// but can also be a UUID for extended box types.
///
/// ```ignore
/// use mp4_bmff::codec::BoxCodec;
/// use mp4_bmff::base::BoxType;
///
/// struct MyBox { /* ... */ }
///
/// impl BoxCodec for MyBox {
///     fn boxtype(&self) -> BoxType {
///         BoxType::new(*b"mybo")
///     }
/// }
/// ```
pub trait BoxCodec {
    /// Returns the box type identifier for this box.
    fn boxtype(&self) -> BoxType;
}

/// Trait for decoding BMFF boxes from byte slices.
///
/// This trait is implemented by box types that can be parsed from binary data.
/// The lifetime parameter `'de` allows zero-copy parsing where the decoded
/// structure can borrow data from the input slice.
///
/// # Input Format
///
/// The `decode` method receives the **payload bytes only**, not including
/// the box header. The header is parsed separately by [`read_box`] before
/// calling `decode`.
///
/// # Lifetime
///
/// - View types (`*View<'de>`) borrow from the input slice
/// - Owned types copy data and can use `'de = 'static`
pub trait BoxDecode<'de>: Sized {
    /// Decodes the box payload from the given byte slice.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The box payload (excluding the header)
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes cannot be parsed as this box type.
    fn decode(bytes: &'de [u8]) -> Result<Self>;
}

/// Trait for encoding BMFF boxes into byte slices.
///
/// This trait is implemented by box types that can be serialized to binary data.
/// It provides methods for calculating the encoded size and writing the payload.
///
/// # Output Format
///
/// The `encode` methods write the **payload bytes only**, not including
/// the box header. Use [`write_box`] to write a complete box with header.
///
/// # Implementation
///
/// Implementors must provide:
/// - `encoded_len()`: Returns the exact payload size in bytes
/// - `encode_into()`: Writes the payload to a buffer
pub trait BoxEncode {
    /// Returns the encoded payload length in bytes (excluding the header).
    ///
    /// This must return the exact number of bytes that `encode_into` will write.
    fn encoded_len(&self) -> usize;

    /// Encodes the box payload into the given byte slice.
    ///
    /// Returns the number of bytes written. This is an internal method;
    /// prefer using [`encode`](Self::encode) or [`encode_to_vec`](Self::encode_to_vec).
    #[doc(hidden)]
    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize>;

    /// Encodes the box payload into the given byte slice.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Buffer to write the payload into
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::NotEnoughBytes`](crate::error::ErrorKind::NotEnoughBytes)
    /// if the buffer is smaller than [`encoded_len`](Self::encoded_len).
    fn encode(&self, bytes: &mut [u8]) -> Result<()> {
        let expected = self.encoded_len();
        let remaining = bytes.len();

        if remaining < expected {
            return Err(Error::new(ErrorKind::NotEnoughBytes {
                expected,
                remaining,
            }));
        }

        let written = self.encode_into(&mut bytes[..expected])?;
        debug_assert!(written == expected);

        Ok(())
    }

    /// Encodes the box payload into a newly allocated `Vec<u8>`.
    ///
    /// This is a convenience method that allocates a buffer of the correct
    /// size and encodes the payload into it.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::codec::BoxEncode;
    /// use mp4_bmff::boxes::bmff::{MfroBox, MfroFlags};
    ///
    /// let mfro = MfroBox {
    ///     version: 0,
    ///     flags: MfroFlags::empty(),
    ///     size: 1234,
    /// };
    /// let payload = mfro.encode_to_vec().unwrap();
    /// assert_eq!(payload.len(), 8); // version(1) + flags(3) + size(4)
    /// ```
    #[cfg(feature = "alloc")]
    fn encode_to_vec(&self) -> Result<Vec<u8>> {
        let len = self.encoded_len();
        let mut buf = crate::lib::vec![0u8; len];
        self.encode(&mut buf)?;

        Ok(buf)
    }
}

/// Calculates box total length helper function.
#[cfg(feature = "alloc")]
pub(crate) fn boxed_len<B>(boxed: &B) -> usize
where
    B: BoxCodec + BoxEncode,
{
    let payload_len = boxed.encoded_len();
    let header = BoxHeader::new(boxed.boxtype(), payload_len as u64);
    header.header_len() + payload_len
}

/// Reads and decodes a complete BMFF box from a byte slice.
///
/// This function parses the box header and payload, returning a decoded
/// box structure. The input must contain at least one complete box.
///
/// # Arguments
///
/// * `bytes` - Byte slice containing a complete box (header + payload)
///
/// # Returns
///
/// The decoded box structure on success.
///
/// # Errors
///
/// - [`ErrorKind::NotEnoughBytes`](crate::error::ErrorKind::NotEnoughBytes):
///   Input is too short for the box header or payload
/// - Other errors from the box's [`BoxDecode::decode`] implementation
///
/// # Example
///
/// ```
/// use mp4_bmff::codec::read_box;
/// use mp4_bmff::boxes::bmff::MfroBox;
///
/// // mfro box: size=16, type="mfro", version=0, flags=0, size=1234
/// let data = [
///     0x00, 0x00, 0x00, 0x10, // size = 16
///     b'm', b'f', b'r', b'o', // type = "mfro"
///     0x00,                   // version = 0
///     0x00, 0x00, 0x00,       // flags = 0
///     0x00, 0x00, 0x04, 0xD2, // size = 1234
/// ];
///
/// let mfro: MfroBox = read_box(&data).unwrap();
/// assert_eq!(mfro.size, 1234);
/// ```
pub fn read_box<'de, B>(bytes: &'de [u8]) -> Result<B>
where
    B: BoxCodec + BoxDecode<'de>,
{
    let mut cur = ReadCursor::new(bytes);
    let b = read_box_in(&mut cur)?;
    Ok(b)
}

/// Encodes and writes a complete BMFF box to a byte slice.
///
/// This function writes the box header followed by the payload. The buffer
/// must be large enough to hold the complete box.
///
/// # Arguments
///
/// * `bytes` - Mutable byte slice to write the box into
/// * `boxed` - The box to encode and write
///
/// # Returns
///
/// The number of bytes written (header + payload) on success.
///
/// # Errors
///
/// - [`ErrorKind::NotEnoughBytes`](crate::error::ErrorKind::NotEnoughBytes):
///   Buffer is too small for the complete box
/// - Other errors from the box's [`BoxEncode::encode`] implementation
///
/// # Example
///
/// ```
/// use mp4_bmff::codec::write_box;
/// use mp4_bmff::boxes::bmff::{MfroBox, MfroFlags};
///
/// let mfro = MfroBox {
///     version: 0,
///     flags: MfroFlags::empty(),
///     size: 1234,
/// };
/// let mut buffer = [0u8; 32];
/// let written = write_box(&mut buffer, &mfro).unwrap();
///
/// // Verify the written data
/// assert_eq!(written, 16); // 8 (header) + 8 (payload)
/// assert_eq!(&buffer[0..4], &[0x00, 0x00, 0x00, 0x10]); // size = 16
/// assert_eq!(&buffer[4..8], b"mfro"); // type
/// ```
pub fn write_box<B>(bytes: &mut [u8], boxed: &B) -> Result<usize>
where
    B: BoxCodec + BoxEncode,
{
    let mut cur = WriteCursor::new(bytes);
    write_box_in(&mut cur, boxed)?;
    Ok(cur.position())
}

pub(crate) fn read_box_in<'de, B>(cur: &mut ReadCursor<'de>) -> Result<B>
where
    B: BoxCodec + BoxDecode<'de>,
{
    let raw = RawBox::parse(cur.inner())?;
    debug_assert!(raw.len() == raw.header().total_size() as usize);
    cur.advance(raw.len())?;

    let b = B::decode(raw.into_payload())?;
    Ok(b)
}

pub(crate) fn write_box_in<B>(cur: &mut WriteCursor<'_>, boxed: &B) -> Result<()>
where
    B: BoxCodec + BoxEncode,
{
    let payload_len = boxed.encoded_len();
    let header = BoxHeader::new(boxed.boxtype(), payload_len as u64);
    let header_len = header.header_len();
    header.write(cur.take_mut(header_len)?)?;

    let buf = cur.take_mut(payload_len)?;
    boxed.encode(buf)?;

    Ok(())
}
