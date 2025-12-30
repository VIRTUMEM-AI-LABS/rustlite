# Contributing to RustLite

First off — thank you for your interest in contributing to RustLite! We welcome contributions of all kinds: bug reports, documentation improvements, examples, benchmarks, and code. This file explains how to get started and what we expect from contributors.

## Key areas where help is needed

- Query engine implementation (parser, planner, executor)
- Transaction coordinator (MVCC, isolation levels)
- Performance benchmarking and optimization
- Full-text indexing (v0.6+)
- Documentation and examples
- Platform-specific optimizations (Windows, Linux, macOS)

## Code of Conduct

Please follow our `CODE_OF_CONDUCT.md`. Be respectful and constructive in all interactions.

## Getting started

1. Fork the repository and clone your fork:

```bash
git clone https://github.com/<your-username>/rustlite.git
cd rustlite
```

2. Create a feature branch for your work:

```bash
git checkout -b feat/my-feature
```

3. Build and run tests:

```bash
cargo build
cargo test
```

4. Run linters or formatters (optional but appreciated):

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## Submitting changes

1. Keep changes small and focused. One logical unit per pull request.
2. Write clear commit messages. Use the imperative mood, e.g., `fix: handle empty key` or `docs: add example for Database`.
3. Push your branch to your fork and open a Pull Request (PR) against `VIRTUMEM-AI-LABS/rustlite:main`.
4. Fill the PR template (if provided) with a short description, motivation, and testing steps.

## Review process

- PRs will be reviewed by maintainers. You may be asked to make changes or expand tests.
- Tests and CI must pass before a PR can be merged.

## Tests and Quality

- Add unit tests for any new behavior where possible (we currently have 135+ tests)
- For performance changes, include benchmark code or reproducible steps.
- Run the full test suite with `cargo test --workspace`

## API Stability and Releases

- We try to keep the public API stable. If you propose breaking changes, open an issue first to discuss design and migration strategy.

## Licensing

By contributing, you agree that your contributions will be licensed under the project's Apache-2.0 license (see `LICENSE`). If you need to contribute under a different license, contact the maintainers first.

## Local development tips

- Use `RUST_LOG=debug cargo test` to see detailed logging during tests.
- Run examples with `cargo run --example <name>` (e.g., `persistent_demo`, `relational_demo`)
- The project currently has:
  - ✅ Persistent storage engine (v0.2+)
  - ✅ Write-Ahead Log (WAL) for durability (v0.2+)
  - ✅ B-Tree and Hash indexing (v0.3+)
  - 🚧 Query engine (v0.4+ planned)
  - 🚧 Transaction coordinator (v0.5+ planned)

## Contact

If you have questions, open an issue or reach out via the project's GitHub Discussions (if enabled) or the contact listed in the `README.md`.

---

Thanks for helping make RustLite better!
