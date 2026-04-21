# tzif-codec

`tzif-codec` is a small Rust crate for parsing, encoding, validating,
and building TZif files as specified by RFC 9636.

The crate keeps its data model close to the binary TZif layout. It is intended
for applications that need to generate TZif bytes, inspect TZif structure, or
round-trip TZif data for use with timezone libraries such as Jiff.

## Scope

- TZif v1, v2, v3, and v4 parsing
- TZif v1, v2, v3, and v4 encoding
- Transition times and transition type indexes
- Local time type records
- Time zone designation tables
- Leap-second records
- Standard/wall and UT/local indicators
- TZif footer strings
- RFC 9636 Appendix A interoperability warnings
- TZDIST media type and truncation helpers

## MSRV

Minimum Supported Rust Version (MSRV): `tzif-codec` supports Rust 1.85.0 and later.

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
tzif-codec = "0.1"
```

The crate currently has no feature flags and uses `std`.

This crate does not implement a timezone engine. It does not expose APIs to
evaluate local time for arbitrary timestamps, resolve DST gaps or folds, or
compile IANA tzdb source files. It does parse POSIX TZ footer strings where RFC
9636 validation requires syntax checks and last-transition consistency checks.
Builders can generate common POSIX TZ footer strings from structured offset and
transition-rule inputs.

This crate also does not implement a TZDIST server or client. It provides the
TZif-specific building blocks a TZDIST implementation needs: media type
identifiers, capability checks, media type validation, and truncation-shape
validation.

## Example

```rust
use tzif_codec::{PosixFooter, PosixTransitionRule, TzifBuilder, TzifFile};

let tzif = TzifBuilder::transitions()
    .designation("PST")
    .designation("PDT")
    .local_time_type("PST", -8 * 3600, false)
    .local_time_type("PDT", -7 * 3600, true)
    .transition(1_710_064_800, "PDT")
    .transition(1_730_624_400, "PST")
    .posix_footer(PosixFooter::daylight_saving(
        "PST",
        -8 * 3600,
        "PDT",
        -7 * 3600,
        PosixTransitionRule::month_weekday(3, 2, 0),
        PosixTransitionRule::month_weekday(11, 1, 0),
    ))
    .build()?;

let bytes = tzif.to_bytes()?;
let parsed = TzifFile::parse(&bytes)?;
assert_eq!(parsed.to_bytes()?, bytes);
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Round-Trip Existing Zoneinfo

Linux systems usually install compiled IANA time zone files under
`/usr/share/zoneinfo`. These files are already TZif, so they can be parsed,
encoded again, written somewhere else, and compared byte-for-byte:

```rust,no_run
use std::{env, fs, path::Path};
use tzif_codec::TzifFile;

let source = Path::new("/usr/share/zoneinfo/Asia/Tokyo");
let output = env::temp_dir().join("Asia_Tokyo.tzif");

let original = fs::read(source)?;
let parsed = TzifFile::parse(&original)?;
let encoded = parsed.to_bytes()?;
fs::write(&output, &encoded)?;

let written = fs::read(&output)?;
assert_eq!(written.len(), original.len());
for (offset, (&expected, &actual)) in original.iter().zip(&written).enumerate() {
    assert_eq!(actual, expected, "byte mismatch at offset {offset}");
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

The TZif header contains 15 reserved bytes. Conforming TZif files write these
bytes as zero, and this crate also writes zeros when encoding, so standard
zoneinfo files are expected to round-trip byte-for-byte.

## Validation

`TzifFile` and `DataBlock` are low-level, mutable representations of the wire
format. Callers that construct or edit them directly can validate RFC 9636
constraints without encoding first:

```rust
use tzif_codec::{DataBlock, TzifFile};

let tzif = TzifFile::v1(DataBlock::placeholder());
tzif.validate()?;
# Ok::<(), tzif_codec::TzifError>(())
```

`to_bytes()`, `parse()`, `validate_for_media_type()`,
`validate_tzdist_truncation()`, and interoperability warnings also run the same
structural validation before using a file.

## Builders

The low-level `TzifFile` and `DataBlock` types are available when callers need
full control over the TZif layout. Most applications should start with
builders, which choose the lowest non-legacy TZif version needed by default and
hide designation table indexes. Version 1 can still be requested explicitly for
legacy interoperability.

```rust
use tzif_codec::TzifBuilder;

let kathmandu = TzifBuilder::fixed_offset("NPT", 5 * 3600 + 45 * 60)
    .build()?;
assert_eq!(kathmandu.footer.as_deref(), Some("NPT-5:45"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Explicit transitions can be used for zones whose transitions have already been
computed by the caller. POSIX TZ footers can be generated from structured
rules, so callers do not need to hand-write strings such as
`EST5EDT,M3.2.0,M11.1.0`.

```rust
use tzif_codec::{PosixFooter, PosixTransitionRule, TzifBuilder};

let eastern = TzifBuilder::transitions()
    .designation("EST")
    .designation("EDT")
    .local_time_type("EST", -5 * 3600, false)
    .local_time_type("EDT", -4 * 3600, true)
    .transition(1_710_054_000, "EDT")
    .transition(1_730_613_600, "EST")
    .posix_footer(PosixFooter::daylight_saving(
        "EST",
        -5 * 3600,
        "EDT",
        -4 * 3600,
        PosixTransitionRule::month_weekday(3, 2, 0),
        PosixTransitionRule::month_weekday(11, 1, 0),
    ))
    .build()?;
assert_eq!(eastern.footer.as_deref(), Some("EST5EDT,M3.2.0,M11.1.0"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Raw footer strings remain available through `.footer(...)` for unusual cases
that the structured `PosixFooter` API does not yet cover. Raw footers are
validated when the resulting `TzifFile` is validated or encoded.

Builder designations are always validated as RFC 9636 designations: ASCII only,
`[A-Za-z0-9+-]`, and 3 to 6 characters. This keeps `build()` from returning a
`TzifFile` that later fails encoding because of designation table data.

## Interoperability Warnings

RFC 9636 Appendix A documents common compatibility issues in older or buggy
TZif readers. Most of these issues do not make a TZif file invalid, so
`tzif-codec` reports them as warnings instead of rejecting the file.

```rust
use tzif_codec::{DataBlock, TzifFile};

let tzif = TzifFile::v3(DataBlock::placeholder(), DataBlock::placeholder(), "<UTC>0");
let warnings = tzif.interoperability_warnings()?;
assert!(!warnings.is_empty());
# Ok::<(), tzif_codec::TzifError>(())
```

The warning API covers statically detectable Appendix A issues such as
incomplete version 1 data, footer-dependent files, version 4 leap-second
tables, missing early no-op transitions, extreme transition timestamps,
non-portable designations, negative DST, leap seconds with sub-minute offsets,
and unusual UTC offsets.

## TZDIST Helpers

RFC 9636 permits TZDIST servers to serve TZif using `application/tzif` and
`application/tzif-leap`. `tzif-codec` keeps HTTP concerns out of scope, but
provides helpers for the TZif-specific rules.

```rust
use tzif_codec::{
    validate_tzdist_capability_formats, TzifMediaType,
};

validate_tzdist_capability_formats([
    TzifMediaType::APPLICATION_TZIF,
    TzifMediaType::APPLICATION_TZIF_LEAP,
])?;
# Ok::<(), tzif_codec::TzdistError>(())
```

Available helpers:

- `TzifMediaType`
- `TzifMediaType::APPLICATION_TZIF`
- `TzifMediaType::APPLICATION_TZIF_LEAP`
- `TzifFile::suggested_media_type`
- `TzifFile::validate_for_media_type`
- `validate_tzdist_capability_formats`
- `TzifFile::validate_tzdist_truncation`
- `TzdistTruncation`

The truncation helper checks the TZif-shape rules from RFC 9636 Section 6.1:
start truncation requires the first version 2+ transition to match the start
point and time type 0 to be a `-00` placeholder; end truncation requires the
last version 2+ transition to match the end point, an empty footer, and the
last transition type to be a `-00` placeholder.

The helper cannot prove that represented information inside a truncation range
matches a corresponding untruncated TZif file. That comparison requires the
untruncated source data and belongs in the TZDIST service implementation.

### Minimal Axum Response Example

The HTTP server, routing, authentication, TLS, ETags, and `Accept` negotiation
belong to the TZDIST implementation. Existing IANA zones can be served by
reading the system zoneinfo file, which is already TZif. Custom zones can be
encoded with `tzif-codec` and returned with the same media type.

```rust,no_run
use axum::{
    body::Body,
    http::{header, HeaderValue, Response, StatusCode},
};
use tzif_codec::{TzifBuilder, TzifMediaType};

async fn zone_response(zone_id: &str) -> Result<Response<Body>, StatusCode> {
    let bytes = if zone_id == "Custom/Office" {
        custom_office_zone()?
    } else {
        read_system_zoneinfo(zone_id).await?
    };

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static(TzifMediaType::APPLICATION_TZIF),
        )
        .body(Body::from(bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn read_system_zoneinfo(zone_id: &str) -> Result<Vec<u8>, StatusCode> {
    if zone_id.starts_with('/') || zone_id.contains("..") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = std::path::Path::new("/usr/share/zoneinfo").join(zone_id);
    tokio::fs::read(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

fn custom_office_zone() -> Result<Vec<u8>, StatusCode> {
    let tzif = TzifBuilder::fixed_offset("WORK", 9 * 3600)
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tzif.validate_for_media_type(TzifMediaType::Tzif)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tzif.to_bytes()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
```

In this example, `Custom/Office` is the application-defined zone identifier used
by the TZDIST service. `WORK` is the TZif local time type designation stored
inside the file. These names do not need to match, but builder designations must
still satisfy the RFC 9636 designation rules described above.

## Conformance

The test suite includes byte-for-byte RFC 9636 Appendix B examples:

- B.1 Version 1 File Representing UTC with Leap Seconds
- B.2 Version 2 File Representing Pacific/Honolulu
- B.3 Truncated Version 2 File Representing Pacific/Johnston
- B.4 Truncated Version 3 File Representing Asia/Jerusalem
- B.5 Truncated Version 4 File Representing Europe/London

Each vector is generated from the Rust data model, parsed back, and
encoded again with byte-for-byte equality.

## Test Strategy

The crate includes unit tests for builders, parsing, encoding,
validation, interoperability warnings, TZDIST helpers, and RFC 9636 Appendix B
byte vectors.

Run the Rust test suite with:

```sh
cargo test
```

Run lint checks with:

```sh
cargo clippy --all-targets -- -D warnings
```

## Code of Conduct

See `CODE_OF_CONDUCT.md`.

## Contributing

See `CONTRIBUTING.md`.

## License

See `LICENSE`.
