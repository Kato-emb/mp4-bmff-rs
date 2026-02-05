use crate::error::*;

use super::size::SizeOfInstance;
use super::tag::Tag;

#[cfg(feature = "alloc")]
use crate::lib::Vec;

/// Raw Descriptor View
#[derive(Debug)]
pub struct RawDescriptor<T> {
    /// Descriptor Tag
    tag: Tag,
    /// Size of the descriptor instance
    size_of_instance: SizeOfInstance,
    /// Descriptor instance bytes
    instance: T,
}

/// A reference to a RawDescriptor's contents.
pub type RawDescriptorRef<'a> = RawDescriptor<&'a [u8]>;

/// An owned RawDescriptor with a `Vec<u8>` instance.
#[cfg(feature = "alloc")]
pub type RawDescriptorOwned = RawDescriptor<Vec<u8>>;

impl<T: Clone> Clone for RawDescriptor<T> {
    fn clone(&self) -> Self {
        Self {
            tag: self.tag,
            size_of_instance: self.size_of_instance,
            instance: self.instance.clone(),
        }
    }
}

impl<T> RawDescriptor<T> {
    /// Returns the descriptor tag
    #[inline]
    pub fn tag(&self) -> Tag {
        self.tag
    }

    /// Returns the size of the descriptor instance
    #[inline]
    pub fn size_of_instance(&self) -> SizeOfInstance {
        self.size_of_instance
    }

    /// Consumes the RawDescriptor, returning the instance bytes
    pub fn into_instance(self) -> T {
        self.instance
    }
}

impl<T: AsRef<[u8]>> RawDescriptor<T> {
    /// Creates a new RawDescriptor
    pub fn new(tag: Tag, instance: T) -> Self {
        let instance_len = instance.as_ref().len() as u32;
        let Some(size_of_instance) = SizeOfInstance::from_u32(instance_len as u32) else {
            panic!("Descriptor instance size exceeds maximum (0x0FFFFFFF)");
        };

        Self {
            tag,
            size_of_instance,
            instance,
        }
    }

    /// Serializes the RawDescriptor into the provided byte slice
    pub fn write(&self, bytes: &mut [u8]) -> Result<()> {
        let total_size = self.len();

        if bytes.len() < total_size {
            return Err(Error::new(ErrorKind::NotEnoughBytes {
                expected: total_size,
                remaining: bytes.len(),
            }));
        }

        let mut offset = 0;

        // write tag
        bytes[offset] = self.tag.0;
        offset += 1;

        // write size_of_instance
        let (size_bytes, size_byte_count) = self.size_of_instance.to_bytes();
        bytes[offset..offset + size_byte_count].copy_from_slice(&size_bytes[..size_byte_count]);
        offset += size_byte_count;

        // write instance
        bytes[offset..total_size].copy_from_slice(self.instance.as_ref());

        Ok(())
    }

    /// Returns the size of the Descriptor when serialized
    #[inline]
    pub fn len(&self) -> usize {
        1 + self.size_of_instance.size_in_bytes() + self.instance.as_ref().len()
    }

    /// Returns true if the Descriptor is empty (has zero length instance).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.instance.as_ref().is_empty()
    }

    /// Returns the instance bytes
    #[inline]
    pub fn instance(&self) -> &[u8] {
        self.instance.as_ref()
    }
}

impl<T: AsMut<[u8]>> RawDescriptor<T> {
    /// Returns the mutable instance bytes
    pub fn instance_mut(&mut self) -> &mut [u8] {
        self.instance.as_mut()
    }
}

impl<'a> RawDescriptor<&'a [u8]> {
    /// Converts the RawDescriptor to an owned version
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> RawDescriptor<Vec<u8>> {
        use crate::lib::Vec;

        RawDescriptor {
            tag: self.tag,
            size_of_instance: self.size_of_instance,
            instance: Vec::from(self.instance),
        }
    }

    /// Parses a RawDescriptor from the given byte slice
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut offset = 0;

        //read tag
        let tag_byte = *bytes.get(offset).ok_or_else(|| {
            Error::new(ErrorKind::NotEnoughBytes {
                expected: 1,
                remaining: bytes.len(),
            })
        })?;
        let tag = Tag(tag_byte);
        offset += 1;

        // read size_of_instance
        let (size_of_instance, size_bytes) = SizeOfInstance::from_bytes(&bytes[offset..])
            .ok_or_else(|| {
                Error::new(ErrorKind::Other {
                    description: "Failed to parse SizeOfInstance",
                })
            })?;
        offset += size_bytes;

        let instance_size = size_of_instance.get() as usize;
        let instance = bytes.get(offset..offset + instance_size).ok_or_else(|| {
            Error::new(ErrorKind::NotEnoughBytes {
                expected: instance_size,
                remaining: bytes.len() - offset,
            })
        })?;

        Ok(RawDescriptor {
            tag,
            size_of_instance,
            instance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_accessors() {
        let instance = [0x01, 0x02, 0x03, 0x04];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &instance[..]);

        assert_eq!(descriptor.tag(), Tag::ES_DESCR_TAG);
        assert_eq!(descriptor.size_of_instance().get(), 4);
        assert_eq!(descriptor.instance(), &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_new_empty_instance() {
        let instance: [u8; 0] = [];
        let descriptor = RawDescriptor::new(Tag::SL_CONFIG_DESCR_TAG, &instance[..]);

        assert_eq!(descriptor.tag(), Tag::SL_CONFIG_DESCR_TAG);
        assert_eq!(descriptor.size_of_instance().get(), 0);
        assert!(descriptor.is_empty());
    }

    #[test]
    fn test_len_one_byte_size() {
        // instance長 < 128 → sizeOfInstance は 1バイト
        let instance = [0u8; 100];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &instance[..]);

        // tag(1) + sizeOfInstance(1) + instance(100) = 102
        assert_eq!(descriptor.len(), 102);
    }

    #[test]
    fn test_len_two_byte_size() {
        // instance長 >= 128 → sizeOfInstance は 2バイト
        let instance = [0u8; 200];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &instance[..]);

        // tag(1) + sizeOfInstance(2) + instance(200) = 203
        assert_eq!(descriptor.len(), 203);
    }

    #[test]
    fn test_is_empty() {
        let empty_instance: [u8; 0] = [];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &empty_instance[..]);
        assert!(descriptor.is_empty());

        let non_empty_instance = [0x01];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &non_empty_instance[..]);
        assert!(!descriptor.is_empty());
    }

    #[test]
    fn test_into_instance() {
        let instance = vec![0x01, 0x02, 0x03];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, instance);

        let recovered = descriptor.into_instance();
        assert_eq!(recovered, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_instance_mut() {
        let mut instance = vec![0x01, 0x02, 0x03];
        let mut descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &mut instance[..]);

        descriptor.instance_mut()[0] = 0xFF;
        assert_eq!(descriptor.instance(), &[0xFF, 0x02, 0x03]);
    }

    #[test]
    fn test_write_small_instance() {
        let instance = [0x01, 0x02, 0x03, 0x04];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &instance[..]);

        let mut buffer = [0u8; 10];
        descriptor.write(&mut buffer).unwrap();

        // tag(0x03) + size(0x04) + instance
        assert_eq!(buffer[0], 0x03); // ES_DESCR_TAG
        assert_eq!(buffer[1], 0x04); // size = 4
        assert_eq!(&buffer[2..6], &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_write_larger_instance() {
        // 128バイトのインスタンス → sizeOfInstance は 2バイト
        let instance = [0xAB; 128];
        let descriptor = RawDescriptor::new(Tag::DECODER_CONFIG_DESCR_TAG, &instance[..]);

        let mut buffer = [0u8; 200];
        descriptor.write(&mut buffer).unwrap();

        // tag(0x04) + size(0x81 0x00) + instance
        assert_eq!(buffer[0], 0x04); // DECODER_CONFIG_DESCR_TAG
        assert_eq!(buffer[1], 0x81); // size high byte (continuation bit set)
        assert_eq!(buffer[2], 0x00); // size low byte
        assert_eq!(&buffer[3..131], &[0xAB; 128]);
    }

    #[test]
    fn test_write_buffer_too_small() {
        let instance = [0x01, 0x02, 0x03, 0x04];
        let descriptor = RawDescriptor::new(Tag::ES_DESCR_TAG, &instance[..]);

        let mut buffer = [0u8; 3]; // too small
        let result = descriptor.write(&mut buffer);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_small_descriptor() {
        // tag(0x03) + size(0x04) + instance(4 bytes)
        let data = [0x03, 0x04, 0x01, 0x02, 0x03, 0x04];

        let descriptor = RawDescriptor::parse(&data).unwrap();

        assert_eq!(descriptor.tag(), Tag::ES_DESCR_TAG);
        assert_eq!(descriptor.size_of_instance().get(), 4);
        assert_eq!(descriptor.instance(), &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn test_parse_two_byte_size() {
        // tag(0x04) + size(0x81 0x00 = 128) + instance(128 bytes)
        let mut data = vec![0x04, 0x81, 0x00];
        data.extend_from_slice(&[0xCD; 128]);

        let descriptor = RawDescriptor::parse(&data).unwrap();

        assert_eq!(descriptor.tag(), Tag::DECODER_CONFIG_DESCR_TAG);
        assert_eq!(descriptor.size_of_instance().get(), 128);
        assert_eq!(descriptor.instance().len(), 128);
    }

    #[test]
    fn test_parse_empty_instance() {
        // tag(0x06) + size(0x00)
        let data = [0x06, 0x00];

        let descriptor = RawDescriptor::parse(&data).unwrap();

        assert_eq!(descriptor.tag(), Tag::SL_CONFIG_DESCR_TAG);
        assert_eq!(descriptor.size_of_instance().get(), 0);
        assert!(descriptor.is_empty());
    }

    #[test]
    fn test_parse_empty_bytes() {
        let data: [u8; 0] = [];
        let result = RawDescriptor::parse(&data);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_size() {
        // tag only, no size
        let data = [0x03];
        let result = RawDescriptor::parse(&data);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_truncated_instance() {
        // tag(0x03) + size(0x10 = 16) + instance(only 4 bytes, should be 16)
        let data = [0x03, 0x10, 0x01, 0x02, 0x03, 0x04];
        let result = RawDescriptor::parse(&data);

        assert!(result.is_err());
    }

    #[test]
    fn test_roundtrip() {
        let original_instance = [0x01, 0x02, 0x03, 0x04, 0x05];
        let original = RawDescriptor::new(Tag::DECODER_SPECIFIC_INFO_TAG, &original_instance[..]);

        // write
        let mut buffer = [0u8; 20];
        original.write(&mut buffer).unwrap();

        // parse
        let parsed = RawDescriptor::parse(&buffer[..original.len()]).unwrap();

        assert_eq!(original.tag(), parsed.tag());
        assert_eq!(
            original.size_of_instance().get(),
            parsed.size_of_instance().get()
        );
        assert_eq!(original.instance(), parsed.instance());
    }

    #[test]
    fn test_roundtrip_large() {
        // 大きなインスタンスでの往復テスト
        let original_instance = [0xFF; 1000];
        let original = RawDescriptor::new(Tag::ES_DESCR_TAG, &original_instance[..]);

        let mut buffer = vec![0u8; original.len()];
        original.write(&mut buffer).unwrap();

        let parsed = RawDescriptor::parse(&buffer).unwrap();

        assert_eq!(original.tag(), parsed.tag());
        assert_eq!(
            original.size_of_instance().get(),
            parsed.size_of_instance().get()
        );
        assert_eq!(original.instance(), parsed.instance());
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_to_owned() {
        let data = [0x03, 0x03, 0x01, 0x02, 0x03];

        let borrowed = RawDescriptor::parse(&data).unwrap();
        let owned = borrowed.to_owned();

        assert_eq!(borrowed.tag(), owned.tag());
        assert_eq!(
            borrowed.size_of_instance().get(),
            owned.size_of_instance().get()
        );
        assert_eq!(borrowed.instance(), owned.instance());
    }
}
