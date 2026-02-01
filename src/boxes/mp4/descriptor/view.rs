use crate::cursor::ReadCursor;
#[cfg(feature = "alloc")]
use crate::cursor::WriteCursor;

use crate::error::*;

use super::SizeOfInstance;
use super::Tag;

/// Base Descriptor View
#[derive(Debug, Clone, Copy)]
pub struct DescriptorView<'a> {
    /// Descriptor Tag
    pub tag: Tag,
    /// Size of the descriptor instance
    pub size_of_instance: SizeOfInstance,
    /// Descriptor instance bytes
    pub instance: &'a [u8],
}

impl<'a> DescriptorView<'a> {
    /// Returns the size of the Descriptor when serialized
    pub fn size(&self) -> usize {
        1 // tag
        + self.size_of_instance.size_in_bytes() // size_of_instance
        + self.instance.len() // instance
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let tag_byte = cur.read_u8()?;
        let tag = Tag(tag_byte);

        let (size_of_instance, size_bytes) = SizeOfInstance::from_bytes(cur.remaining_slice())
            .ok_or(Error::at(
                ErrorKind::Other {
                    description: "Failed to parse SizeOfInstance",
                },
                cur.position() as u64,
            ))?;
        cur.advance(size_bytes)?;

        let instance_size = size_of_instance.get() as usize;
        let instance = cur.take(instance_size)?;

        Ok(DescriptorView {
            tag,
            size_of_instance,
            instance,
        })
    }

    #[cfg(feature = "alloc")]
    pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
        cur.write_u8(self.tag.0)?;

        let (size_bytes, byte_count) = self.size_of_instance.to_bytes();
        cur.write_slice(&size_bytes[..byte_count])?;

        cur.write_slice(self.instance)?;

        Ok(())
    }
}

#[cfg(feature = "alloc")]
pub use owned::DescriptorOwned;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    /// Owned Descriptor
    #[derive(Debug, Clone)]
    pub struct DescriptorOwned {
        /// Descriptor Tag
        pub tag: Tag,
        /// Size of the descriptor instance
        pub size_of_instance: SizeOfInstance,
        /// Descriptor instance bytes
        pub instance: Vec<u8>,
    }

    impl DescriptorOwned {
        /// Returns the size of the Descriptor when serialized
        pub fn size(&self) -> usize {
            1 // tag
            + self.size_of_instance.size_in_bytes() // size_of_instance
            + self.instance.len() // instance
        }

        /// Creates an owned descriptor from a view
        pub fn from_view(view: &DescriptorView) -> Self {
            Self {
                tag: view.tag,
                size_of_instance: view.size_of_instance,
                instance: view.instance.to_vec(),
            }
        }

        /// Converts the owned descriptor into a view
        pub fn to_view(&self) -> DescriptorView<'_> {
            DescriptorView {
                tag: self.tag,
                size_of_instance: self.size_of_instance,
                instance: &self.instance,
            }
        }
    }

    impl From<DescriptorView<'_>> for DescriptorOwned {
        fn from(view: DescriptorView<'_>) -> Self {
            Self::from_view(&view)
        }
    }

    impl DescriptorView<'_> {
        /// Converts the view into an owned descriptor
        pub fn to_owned(&self) -> DescriptorOwned {
            DescriptorOwned::from_view(self)
        }
    }
}
