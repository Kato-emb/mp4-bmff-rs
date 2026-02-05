//! MPEG-4 Systems (ISO/IEC 14496-1) structures and types.
//!
//! This module provides descriptor structures defined in the MPEG-4 Systems
//! specification (ISO/IEC 14496-1). These descriptors are used to convey
//! elementary stream configuration and synchronization information.
//!
//! # Submodules
//!
//! - [`descriptor`]: MPEG-4 descriptor types including:
//!   - [`descriptor::EsDescriptorView`]: Elementary Stream Descriptor
//!   - [`descriptor::DecoderConfigDescriptorView`]: Decoder Configuration
//!   - [`descriptor::RawDescriptor`]: Generic descriptor representation

pub mod descriptor;
