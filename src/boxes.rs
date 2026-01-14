//! BMFF box core types and traits.
//!
//! This module contains the core types and traits used to represent and manipulate
//! BMFF boxes in a strongly-typed manner.

pub mod error;

pub mod boxsize;
pub mod boxtype;
pub mod header;
pub mod iter;
pub mod view;
