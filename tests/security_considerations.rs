#![allow(
    clippy::indexing_slicing,
    clippy::too_many_arguments,
    reason = "integration tests use generated fixtures and boundary mutation"
)]

mod common;

mod security_considerations {
    mod footer;
    mod layout;
    mod leap_seconds;
    mod parser;
    mod support;
    mod validation;
}
