//! Serializer and deserializer for the Time Zone Information Format (`TZif`).
//!
//! This crate focuses on the RFC 9636 binary interchange format. It keeps the
//! data model close to the on-the-wire layout so callers can build, inspect,
//! deserialize, and serialize `TZif` files without pulling in a timezone engine.
//!
//! ```compile_fail
//! let _ = tzif_codec::APPLICATION_TZIF;
//! ```

mod builder;
mod common;
mod error;
mod footer;
mod interop;
mod leap;
mod model;
mod parse;
mod tzdist;
mod validate;
mod write;

pub use builder::{
    ExplicitTransitionsBuilder, FixedOffsetBuilder, PosixFooter, PosixTransitionRule,
    PosixTransitionTime, TzifBuilder, VersionPolicy,
};
pub use error::{TzdistError, TzifBuildError, TzifError};
pub use interop::InteroperabilityWarning;
pub use model::{DataBlock, LeapSecond, LocalTimeType, TzifFile, Version};
pub use tzdist::{validate_tzdist_capability_formats, TzdistTruncation, TzifMediaType};
