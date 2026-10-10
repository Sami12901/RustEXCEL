# Building & Packaging Rust EXEL

This guide outlines how to build, test, and package Rust EXEL across Windows, Linux, and macOS.

---

## 1. Prerequisites

- **Rust Toolchain**: Stable Rust 1.85.0 or later (verified on Rust 1.99.0).
  Install via [rustup.rs](https://rustup.rs/):
  ```bash
  rustup update stable
  ```
- **Cargo**: Packaged automatically with the official Rust toolchain.
- **Git**: For source version control and submodule access.

---

## 2. Platform-Specific Prerequisites

### Windows
- Windows 10, Windows 11, or Windows Server 2019+.
- Microsoft C++ Build Tools (MSVC) from Visual Studio 2022 or Build Tools installer.
- DirectX 12 / Vulkan compatible graphics drivers (or software rasterizer via Glow fallback).
- Build command:
  ```powershell
  cargo build --release
  ```

### Linux (Debian / Ubuntu / Fedora / Arch)
On Linux, `egui` and `wgpu` require native X11/Wayland windowing, font rendering, and audio dev packages:

#### Ubuntu / Debian:
```bash
sudo apt-get update
sudo apt-get install -y \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libfontconfig1-dev \
    libasound2-dev \
    libssl-dev \
    libgl1-mesa-dev
```

#### Fedora / RHEL:
```bash
sudo dnf install -y \
    libxcb-devel \
    libxkbcommon-devel \
    fontconfig-devel \
    alsa-lib-devel \
    openssl-devel \
    mesa-libGL-devel
```

#### Arch Linux:
```bash
sudo pacman -S --needed \
    libxcb \
    libxkbcommon \
    fontconfig \
    alsa-lib \
    openssl \
    mesa
```

### macOS
- macOS 12 (Monterey) or later (Apple Silicon M1/M2/M3/M4 or Intel x86_64).
- Xcode Command Line Tools:
  ```bash
  xcode-select --install
  ```
- Build command:
  ```bash
  cargo build --release
  ```

---

## 3. Building the Targets

### Native Desktop GUI Application
```bash
# Debug build (fast compile)
cargo build --bin rust-exel

# Release build (optimized)
cargo build --release --bin rust-exel
```
The binary will be located at `target/release/rust-exel` (or `rust-exel.exe` on Windows).

### Headless CLI Utility
```bash
cargo build --release --bin rust-exel-cli
```
The CLI binary will be located at `target/release/rust-exel-cli` (or `rust-exel-cli.exe` on Windows).

---

## 4. Running the Binaries

### Launch GUI Desktop App
```bash
cargo run --release --bin rust-exel
```

### Headless CLI Commands
```bash
# Evaluate formula expression
cargo run --bin rust-exel-cli -- eval "SUM(10, 20, 30) * 1.5"

# Convert XLSX to CSV
cargo run --bin rust-exel-cli -- convert financial_model.xlsx output.csv

# Inspect sheet structure and metadata
cargo run --bin rust-exel-cli -- info financial_model.xlsx

# Headless recalculation pass
cargo run --bin rust-exel-cli -- recalc input.xlsx -o recalculated.xlsx
```

---

## 5. Graphics & Renderer Configuration

Rust EXEL features automatic renderer fallback:
- **Default**: Initializes `wgpu` with native hardware acceleration (DirectX 12 on Windows, Metal on macOS, Vulkan/X11 on Linux).
- **Automatic Fallback**: If `wgpu` initialization encounters driver incompatibilities or running in a virtualized container, the runtime automatically falls back to OpenGL via `glow`.
- **Diagnostics**: Any graphics initialization failures or unexpected exceptions are captured and written to `crash.log` in the application directory.
- **Manual Override**: You can force software or specific backends via environment variables:
  ```bash
  # Force OpenGL backend
  WGPU_BACKEND=gl cargo run --bin rust-exel

  # Force Vulkan backend
  WGPU_BACKEND=vulkan cargo run --bin rust-exel
  ```

---

## 6. Verification & Quality Assurance

```bash
# Run full automated workspace test suite
cargo test --workspace

# Check formatting compliance
cargo fmt --all -- --check

# Check strict workspace lints (deny warnings)
cargo clippy --workspace --all-targets -- -D warnings
```
