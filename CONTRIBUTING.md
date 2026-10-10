# Contributing to Rust EXEL

Thank you for your interest in contributing to Rust EXEL!

## Development Workflow
1. Ensure you have Rust stable installed (`rustc --version >= 1.85.0`).
2. Fork and clone the repository.
3. Create a descriptive feature branch: `git checkout -b feature/my-feature`.
4. Run formatting and linting:
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets -- -D warnings
   ```
5. Ensure all workspace tests pass:
   ```bash
   cargo test --workspace
   ```
6. Submit a Pull Request with a clear description and testing notes.
