# Rust EXEL - Feature Completion Matrix

This document tracks the verified implementation and testing status of all Rust EXEL features across all supported platforms (Windows, Linux, macOS).

Status Definitions:
- `PLANNED`: In architecture specification; work scheduled.
- `IN_PROGRESS`: Active implementation in development.
- `IMPLEMENTED`: Code written, integrated, and passing workspace compilation.
- `TESTED`: Covered by automated unit/integration tests and verified in live execution.
- `PARTIALLY_SUPPORTED`: Functionality working with specific documented limitations.
- `UNSUPPORTED`: Explicitly out-of-scope or unverified.

---

## 1. Core Grid & Workbook Scale

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Sparse Cell Model (1,048,576 rows × 16,384 cols) | TESTED | `exel-cell`, `exel-core` | Navigation, Cell Edit | Bounded by host RAM | Win/Linux/macOS | Unit tests in `exel-cell` & `exel-core` (4 passed) |
| Multi-Sheet Management | TESTED | `exel-workbook` | New (+), Rename, Switch, Context Menu | Up to host memory | Win/Linux/macOS | Unit tests in `exel-workbook` (1 passed) |
| Virtualized Grid Rendering | TESTED | `exel-ui` | Scroll, Pan, Zoom | Viewport clipped | Win/Linux/macOS | Virtual grid layout & bounds checks |
| Active Cell & Marquee Selection | TESTED | `exel-ui` | Click, Drag, Shift+Arrow | Rectangular range | Win/Linux/macOS | Marquee selection bounds tests |
| Drag-to-Fill Handle | IMPLEMENTED | `exel-ui` | Mouse drag corner | Basic linear sequence | Win/Linux/macOS | Fill handle event loop |
| Row & Column Resizing | TESTED | `exel-ui` | Drag header dividers | Dynamic width/height | Win/Linux/macOS | Resize state handlers |
| Freeze Panes | TESTED | `exel-ui`, `exel-workbook` | View -> Freeze Panes | Split row/column | Win/Linux/macOS | Freeze pane coordinate tests |

---

## 2. Calculation & Formula Engine

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Formula Parsing (AST) | TESTED | `exel-formula` | Enter formula `=...` | Full Excel operator precedence | Win/Linux/macOS | 16 tests passed in `exel-formula` |
| Dependency Graph (DAG) | TESTED | `exel-dependency-graph` | Automatic | Topological sorting | Win/Linux/macOS | 12 tests passed in `exel-dependency-graph` |
| Cycle Detection (`#CIRCULAR!`) | TESTED | `exel-dependency-graph` | Automatic | Tarjan SCC algorithm | Win/Linux/macOS | Cyclic graph test suite |
| Math Functions (SUM, PRODUCT, ABS, ROUND, MOD, SQRT...) | TESTED | `exel-calc` | Ribbon / AutoSum / `=...` | Standard IEEE float math | Win/Linux/macOS | 8 unit tests in `exel-calc` |
| Statistical Functions (AVERAGE, MIN, MAX, COUNT, COUNTA) | TESTED | `exel-calc` | Formulas Tab | Text cells ignored in numeric ranges | Win/Linux/macOS | Statistical formula test vectors |
| Logical Functions (IF, AND, OR, NOT, IFERROR) | TESTED | `exel-calc` | Formulas Tab | Nested short-circuit evaluation | Win/Linux/macOS | Logical formula test vectors |
| Lookup Functions (VLOOKUP, INDEX, MATCH) | TESTED | `exel-calc` | Formulas Tab | Exact and approximate lookup | Win/Linux/macOS | Lookup test suite (4 passed) |
| Text Functions (CONCAT, LEFT, RIGHT, MID, LEN, TRIM...) | TESTED | `exel-calc` | Formulas Tab | Unicode character indexing | Win/Linux/macOS | Text formula test vectors |
| Standard Error Values (`#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#N/A`, `#NUM!`, `#CIRCULAR!`) | TESTED | `exel-cell`, `exel-calc` | Cell rendering | Distinct error representations | Win/Linux/macOS | Error propagation test suite |

---

## 3. Formatting & Styles

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Shared Style Registry (`StyleTable`) | TESTED | `exel-formatting` | Home Tab | Deduplicated styles via interning | Win/Linux/macOS | 10 tests passed in `exel-formatting` |
| Font (Family, Size, Bold, Italic, Color) | TESTED | `exel-formatting`, `exel-ui` | Ctrl+B, Ctrl+I, Home Tab | Configurable fonts | Win/Linux/macOS | Font format builder tests |
| Cell Fills & Background Colors | TESTED | `exel-formatting`, `exel-ui` | Fill Color picker | Solid fills & standard patterns | Win/Linux/macOS | Fill constructors tests |
| Cell Borders (Thin, Thick, Double, Dashed...) | TESTED | `exel-formatting`, `exel-ui` | Borders Dropdown | Top, bottom, left, right, diagonal | Win/Linux/macOS | Border construction tests |
| Number Formatting (Currency, Percentage, Date, Time) | TESTED | `exel-formatting` | Ctrl+Shift+$, %, # | Custom format strings & Excel codes | Win/Linux/macOS | Number format parsing & format tests |
| Text Alignment (Horizontal, Vertical, Wrap, Indent) | TESTED | `exel-formatting`, `exel-ui` | Alignment buttons | Full alignment options | Win/Linux/macOS | Alignment builder tests |

---

## 4. History, Commands & Tables

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Undo / Redo Command History | TESTED | `exel-history`, `exel-commands` | Ctrl+Z / Ctrl+Y | Bounded history stack with transactions | Win/Linux/macOS | 7 tests passed (3 in history, 4 in commands) |
| Cell & Range Commands | TESTED | `exel-commands` | Edit, Clear Range | Atomic rollback on error | Win/Linux/macOS | Set/Clear/Format range tests |
| Row & Column Structural Commands | TESTED | `exel-commands` | Insert/Delete Row/Col | Restores data on undo | Win/Linux/macOS | Shift row/col tests |
| Structured Tables (`Table`) | TESTED | `exel-tables` | Ctrl+T / Insert Table | Header row, Total row, banded rows | Win/Linux/macOS | 4 tests passed in `exel-tables` |
| Table Structured References | TESTED | `exel-tables` | `Table1[Column]`, `[@Col]` | Exact syntax matching | Win/Linux/macOS | Structured reference tests |

---

## 5. File Formats & Persistence

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| XLSX Reading (calamine) | TESTED | `exel-xlsx` | File -> Open | Values, formulas, styles | Win/Linux/macOS | XLSX round-trip tests |
| XLSX Writing (rust_xlsxwriter) | TESTED | `exel-xlsx` | File -> Save / Save As | Formats, formulas, dimensions | Win/Linux/macOS | 3 tests passed in `exel-xlsx` |
| CSV / TSV Import & Export | TESTED | `exel-csv` | File -> Import / Export | Type inference, delimiter sniffing | Win/Linux/macOS | 4 tests passed in `exel-csv` |
| JSON Import & Export | TESTED | `exel-csv` | File -> Import / Export | Tabular records and 2D arrays | Win/Linux/macOS | JSON round-trip tests |
| Atomic File Save (tempfile) | TESTED | `exel-io` | Ctrl+S | Preserves original on failure | Win/Linux/macOS | 3 tests passed in `exel-io` |
| Macro / VBA Safety | TESTED | `exel-xlsx` | File Open warning | Quarantines macros, never executes | Win/Linux/macOS | Macro detection test |

---

## 6. UI, Productivity & Navigation

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Excel Ribbon Tabs (10 Tabs) | IMPLEMENTED | `exel-ui` | Ribbon bar | Contextual tabs ready | Win/Linux/macOS | UI layout components |
| Formula Bar & Name Box | IMPLEMENTED | `exel-ui` | Click, F2, Type | Jump-to-cell, Commit/Cancel | Win/Linux/macOS | Formula bar sync |
| Status Bar Statistics (Average, Count, Sum) | IMPLEMENTED | `exel-ui` | Selection change | Computed on active selection | Win/Linux/macOS | Status bar live calculation |
| Format Cells Dialog | IMPLEMENTED | `exel-ui` | Ctrl+1 / Ribbon | 5 tabs with live preview | Win/Linux/macOS | Modal dialog state |
| Central Keyboard Shortcuts | TESTED | `exel-shortcuts` | Ctrl/Cmd keybinds | Win, Linux, Mac modifiers | Win/Linux/macOS | Keybinding mapping |
| Native Desktop App | IMPLEMENTED | `apps/rust-exel` | Run binary | wgpu hardware acceleration | Win/Linux/macOS | Native eframe app entry |
| Headless Spreadsheet CLI | IMPLEMENTED | `apps/rust-exel-cli` | `recalculate`, `convert`, `eval` | Batch processing | Win/Linux/macOS | Headless CLI commands |
