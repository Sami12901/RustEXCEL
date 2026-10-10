# Testing Strategy & Quality Assurance - Rust EXEL

This document outlines the testing strategy, test coverage distribution, and continuous verification procedures for the Rust EXEL workspace.

---

## 1. Quality & Verification Philosophy

1. **Computational Fidelity**: Mathematical, statistical, logical, lookup, and text functions are tested against verified spreadsheet evaluation vectors, guaranteeing Excel formula semantics.
2. **Round-Trip Persistence Integrity**: Saving to XLSX, CSV, TSV, or JSON and reopening must preserve cell coordinates, raw values, formulas, and formatting without degradation or data loss.
3. **Sparse Scaling & Memory Safety**: Worksheets are validated with coordinates up to the standard Excel bounds (Row 1,048,576 and Column 16,384 `XFD`) to ensure that non-populated cells occupy zero memory overhead.
4. **Resilience & Fault Tolerance**: File write operations execute through atomic temporary files (`tempfile`), invalid formulas degrade safely into distinct Excel errors (`#DIV/0!`, `#VALUE!`, `#CIRCULAR!`), and graphics failures trigger automatic backend fallbacks (`wgpu` -> `glow`).

---

## 2. Test Suite Breakdown

The workspace features **295 automated unit and integration tests** across 25 crates and applications:

| Crate / Target | Tests | Primary Focus |
|---|---|---|
| `crates/exel-core` | 17 | Coordinate math, `CellCoord`, `CellRange`, A1 reference conversions, bounds |
| `crates/exel-cell` | 18 | `SparseCellStore`, `CellValue`, `CellState`, error type semantics |
| `crates/exel-workbook` | 10 | Multi-sheet management, sheet ordering, named ranges, active sheet state |
| `crates/exel-dependency-graph` | 20 | DAG edges, Kahn's topological sort, Tarjan's SCC circular reference detection |
| `crates/exel-formula` | 43 | Tokenizer, AST recursive-descent parser, operator precedence, cross-sheet refs |
| `crates/exel-calc` | 37 | Formula evaluator, math/stat/logic/lookup/text function registry, recalc |
| `crates/exel-formatting` | 11 | `StyleTable` interning, font, fill, border, alignment, Excel format codes |
| `crates/exel-commands` | 10 | Concrete commands (`SetCell`, `InsertRow`, `DeleteCol`, `FormatRange`), rollback |
| `crates/exel-history` | 10 | Undo/redo stack, bounded capacity, transaction commit/rollback |
| `crates/exel-tables` | 10 | Structured tables, unique column names, structured references, totals |
| `crates/exel-validation` | 12 | Criteria validation (whole number, decimal, list, date range, text length) |
| `crates/exel-clipboard` | 7 | RFC 4180-compliant TSV serializing, parsing, cell mapping, arboard integration |
| `crates/exel-io` | 4 | Format detection, atomic file replacement with tempfile, error handling |
| `crates/exel-csv` | 16 | CSV/TSV sniffing, quote escaping, data type inference, JSON round-tripping |
| `crates/exel-xlsx` | 3 | Calamine reading, `rust_xlsxwriter` writing, styles, formulas, multi-sheet |
| `crates/exel-shortcuts` | 14 | Cross-platform keybinding matching (Windows, Linux, macOS) |
| `crates/exel-charts` | 7 | Bar, line, pie, and scatter series extraction and chart styling |
| `crates/exel-pivot` | 14 | Pivot aggregations (Sum, Count, Avg, Min, Max), row/col grand totals |
| `crates/exel-data` | 9 | Sorting stability, single/multi-key sorting, quick filtering predicates |
| `crates/exel-print` | 6 | Page setup, paper dimensions (Letter, A4, etc.), margins, print preview |
| `crates/exel-automation` | 7 | Scriptable batch runner (SetCell, InsertRow, Recalculate, ExportFile) |
| `crates/exel-ui` | 1 | Virtualized grid widget and layout primitives |
| `apps/rust-exel` | 9 | GUI file persistence (.xlsx, .csv, .tsv, .json), window title, structural row/col edits |
| `apps/rust-exel-cli` | 10 | Headless CLI commands (`eval`, `convert`, `info`, `recalc`, argument parsing) |
| **Total** | **295** | **100% Passing** |

---

## 3. Running Tests Locally

### Run All Workspace Tests
```bash
cargo test --workspace
```

### Run Tests for Specific Components
```bash
# GUI persistence regression tests
cargo test -p rust-exel --lib

# Formula engine and dependency graph tests
cargo test -p exel-calc
cargo test -p exel-formula
cargo test -p exel-dependency-graph

# Clipboard tests
cargo test -p exel-clipboard

# Headless CLI tests
cargo test -p rust-exel-cli
```

### Linting & Formatting Verification
```bash
# Verify formatting
cargo fmt --all -- --check

# Strict workspace lints with warnings treated as errors
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 4. Continuous Integration (CI)

The GitHub Actions CI pipeline (`.github/workflows/ci.yml`) runs on all pushes and pull requests targeting `main` and `master`:
1. **Format & Lint Job (`ubuntu-latest`)**:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
2. **Cross-Platform Test Matrix (`ubuntu-latest`, `windows-latest`, `macos-latest`)**:
   - Installs native Linux windowing/graphics dependencies on Ubuntu (`libxcb-render0-dev`, `libxkbcommon-dev`, etc.)
   - Runs `cargo test --workspace --verbose` across all three operating systems.
