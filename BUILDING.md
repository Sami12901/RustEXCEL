# Building Rust EXEL

## Prerequisites
- Rust stable 1.85.0 or later (verified on Rust 1.99.0)
- Cargo (packaged with Rust)

### Platform-Specific Dependencies

#### Windows
No extra system dependencies are required. Standard MSVC C++ build tools are recommended.
```powershell
cargo build --release
```

#### Linux
On Debian/Ubuntu-based distributions, install the required graphics and windowing libraries:
```bash
sudo apt-get update
sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libfontconfig1-dev libasound2-dev libssl-dev
```

#### macOS
Xcode Command Line Tools are required:
```bash
xcode-select --install
cargo build --release
```

## Running the Application
```bash
# Run the native desktop application
cargo run --bin rust-exel

# Run the headless CLI tool
cargo run --bin rust-exel-cli -- --help
```
