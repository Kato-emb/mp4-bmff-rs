/// Macro to define BoxType constants in a concise, table-like format.
macro_rules! define_box_types {
    ($(
        $(#[$meta:meta])*
        $name:ident = $fourcc:literal
    ),* $(,)?) => {
        impl crate::BoxType {
            $(
                $(#[$meta])*
                pub const $name: Self = Self {
                    boxtype: crate::types::FourCC::new(*$fourcc),
                    usertype: None,
                };
            )*
        }
    };
}

/// Macro to define box-specific flag types with named flags.
///
/// # Example
///
/// ```ignore
/// define_box_flags!(
///     /// Flags for Track Header Box.
///     TkhdFlags {
///         /// Track is enabled.
///         TRACK_ENABLED = 0x000001,
///         /// Track is used in the movie.
///         TRACK_IN_MOVIE = 0x000002,
///     }
/// );
///
/// let flags = TkhdFlags::TRACK_ENABLED | TkhdFlags::TRACK_IN_MOVIE;
/// assert!(flags.contains(TkhdFlags::TRACK_ENABLED));
/// ```
macro_rules! define_box_flags {
    (
        $(#[$type_meta:meta])*
        $name:ident {
            $(
                $(#[$flag_meta:meta])*
                $flag:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        $(#[$type_meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u32);

        impl $name {
            $(
                $(#[$flag_meta])*
                pub const $flag: Self = Self($value & 0x00FF_FFFF);
            )*

            /// Creates an empty flag set with no flags set.
            #[inline]
            pub const fn empty() -> Self { Self(0) }

            /// Creates a flag set with all 24 bits set.
            #[inline]
            pub const fn all() -> Self { Self(0x00FF_FFFF) }

            /// Creates a new flag set from raw flags, masking to 24 bits.
            #[inline]
            pub const fn new(raw_flags: u32) -> Self { Self(raw_flags & 0x00FF_FFFF) }

            /// Returns the raw flags value as a `u32`.
            #[inline]
            pub const fn bits(&self) -> u32 { self.0 }

            /// Returns `true` if no flags are set.
            #[inline]
            pub const fn is_empty(&self) -> bool { self.0 == 0 }

            /// Returns `true` if all flags in `other` are set in `self`.
            #[inline]
            pub const fn contains(&self, other: Self) -> bool { (self.0 & other.0) == other.0 }

            /// Returns `true` if any flags in `other` are set in `self`.
            #[inline]
            pub const fn intersects(&self, other: Self) -> bool { (self.0 & other.0) != 0 }

            /// Inserts the specified flags.
            #[inline]
            pub fn insert(&mut self, other: Self) { self.0 |= other.0; }

            /// Removes the specified flags.
            #[inline]
            pub fn remove(&mut self, other: Self) { self.0 &= !other.0; }

            /// Toggles the specified flags.
            #[inline]
            pub fn toggle(&mut self, other: Self) { self.0 ^= other.0; }

            /// Sets or removes the specified flags depending on `value`.
            #[inline]
            pub fn set(&mut self, other: Self, value: bool) {
                if value {
                    self.insert(other);
                } else {
                    self.remove(other);
                }
            }

            /// Creates a new flag set from a 3-byte big-endian array.
            #[inline]
            pub const fn from_be_bytes(bytes: [u8; 3]) -> Self {
                Self(((bytes[0] as u32) << 16) | ((bytes[1] as u32) << 8) | (bytes[2] as u32))
            }

            /// Returns the flags as a 3-byte big-endian array.
            #[inline]
            pub const fn to_be_bytes(self) -> [u8; 3] {
                [
                    ((self.0 >> 16) & 0xFF) as u8,
                    ((self.0 >> 8) & 0xFF) as u8,
                    (self.0 & 0xFF) as u8,
                ]
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({:#08X})", stringify!($name), self.0)
            }
        }

        impl Default for $name {
            #[inline]
            fn default() -> Self { Self::empty() }
        }

        impl From<u32> for $name {
            #[inline]
            fn from(value: u32) -> Self { Self::new(value) }
        }

        impl From<$name> for u32 {
            #[inline]
            fn from(value: $name) -> Self { value.0 }
        }

        impl core::ops::Not for $name {
            type Output = Self;

            #[inline]
            fn not(self) -> Self::Output { Self(!self.0 & 0x00FF_FFFF) }
        }

        impl core::ops::BitOr for $name {
            type Output = Self;

            #[inline]
            fn bitor(self, rhs: Self) -> Self::Output { Self(self.0 | rhs.0) }
        }

        impl core::ops::BitOrAssign for $name {
            #[inline]
            fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
        }

        impl core::ops::BitAnd for $name {
            type Output = Self;

            #[inline]
            fn bitand(self, rhs: Self) -> Self::Output { Self(self.0 & rhs.0) }
        }

        impl core::ops::BitAndAssign for $name {
            #[inline]
            fn bitand_assign(&mut self, rhs: Self) { self.0 &= rhs.0; }
        }

        impl core::ops::BitXor for $name {
            type Output = Self;

            #[inline]
            fn bitxor(self, rhs: Self) -> Self::Output { Self(self.0 ^ rhs.0) }
        }

        impl core::ops::BitXorAssign for $name {
            #[inline]
            fn bitxor_assign(&mut self, rhs: Self) { self.0 ^= rhs.0; }
        }
    };
}
