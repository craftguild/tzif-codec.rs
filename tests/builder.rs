#![allow(
    clippy::indexing_slicing,
    reason = "integration tests use fixture indexing"
)]

mod common;

mod builder {
    mod explicit_transitions;
    mod fixed_offset;
    mod posix_footer;
}
