use crate::cursor::ReadCursor;

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
        /// Creates an owned descriptor from a view
        pub fn from_view(view: &DescriptorView) -> Self {
            Self {
                tag: view.tag,
                size_of_instance: view.size_of_instance,
                instance: view.instance.to_vec(),
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
