# Contributing to CineSched

Thanks for your interest in contributing. CineSched is a gift to the film production community and welcomes contributions of all kinds - bug fixes, features, documentation, and ports to other platforms.

## Getting Started

### Requirements
- Stable Rust toolchain (Rustup recommended)
- Windows or macOS for the Rust desktop application
- Xcode 14.0 or later only when working on the legacy SwiftUI implementation

### Setup
1. Fork and clone the repository
2. Run `cargo run` to start the Rust desktop application
3. Run `cargo fmt --all -- --check`, `cargo build --all-targets`, and `cargo test --all-targets`
4. Run `cargo clippy --all-targets --all-features -- -D warnings` before submitting changes

The Rust port is in progress. The SwiftUI files document the existing behavior
and should not be removed until their functionality has been migrated and
verified on Windows and macOS.

## Project Structure

The active Rust application is under `src/`. The SwiftUI implementation remains
at the repository root as the feature-parity reference during migration.

```
cinesched/
├── Cargo.toml
├── src/
│   ├── lib.rs                 # Project persistence and core tests
│   ├── main.rs                # egui desktop application
│   ├── model.rs               # Production, schedule, scene, and call-sheet models
│   ├── parsing.rs             # Page and time input parsing
│   ├── reports.rs             # Schedule and production PDF reports
│   └── script_import.rs       # Final Draft, Fountain, and Highland import
└── .github/workflows/rust.yml # Windows and macOS build/test workflow
```

## How to Contribute

### Reporting Bugs
Open an issue with:
- Operating system and Rust toolchain version
- Steps to reproduce
- What you expected vs. what happened
- Console output if relevant

### Submitting a Pull Request
1. Create a branch from `main` with a descriptive name (e.g. `feature/call-sheets` or `fix/pdf-grid-lines`)
2. Keep changes focused - one feature or fix per PR
3. Follow idiomatic Rust style and keep each file focused
4. Run the Rust build and unit tests; verify platform-specific changes on Windows and macOS
5. Update `CHANGELOG.md` under an `[Unreleased]` section

### Code Style
- Idiomatic Rust and Rustdoc for public APIs
- New features should live in their own file where it makes sense
- Avoid adding logic to `ContentView.swift` - use extensions or new files

## Project File Format

CineSched saves projects as standard `.json` files. The format is intentionally simple and human-readable. Any contributions that touch the data model should:
- Maintain backwards compatibility with existing save files
- Use optional fields with sensible defaults for any new properties
- Document the migration path in the PR description

The JSON format is also designed to be portable - a Windows or Linux version of CineSched should be able to read and write the same files.

## Rust Migration Status

A Windows-compatible Rust version is under active development. Keep the
existing `.json` project format readable and writable, and retain unknown
fields when porting additional model data. The remaining feature-parity work
is documented in the README. Keep the SwiftUI reference until the new
implementation has been verified against it on Windows and macOS.

The Rust application uses egui and targets both Windows and macOS.

## Feature Ideas

Remaining parity and feature areas:

- [ ] Drag-and-drop scene scheduling and multi-select editing
- [ ] Localized English and Spanish UI
- [ ] Advanced call-sheet weather and sun-time details
- [ ] Full vector PDF styling and page layout parity
- [ ] Multi-select scenes for batch editing
- [ ] Recent-project management and autosave
- [ ] Complete undo/redo coverage
- [ ] iCloud sync and CSV / Excel export

## License

By contributing, you agree that your contributions will be licensed under the GNU General Public License v3. This means any modified versions of CineSched - including ports to other platforms - must also be released as open source under the same license.
