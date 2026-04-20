# Contributing

Thanks for your interest in contributing.

## General workflow

- Fork the repository.
- Create a branch in your fork.
- Make your changes in your fork.
- Add or update unit tests for your changes.
- Run the tests and confirm there are no errors.
- Open a Pull Request from your fork.

## Tests

Run:

```bash
cargo test
```

If you changed public API, validation behavior, parsing behavior, or serialization behavior, also run:

```bash
cargo clippy --all-targets -- -D warnings
```
