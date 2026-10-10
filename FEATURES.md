# Rust EXEL - Feature Completion Matrix

This document tracks the verified implementation and testing status of all Rust EXEL features across all supported platforms (Windows, Linux, macOS).

Status Definitions:
- `TESTED`: Verified by automated unit/integration tests and verified in live execution.
- `IMPLEMENTED`: Code written, fully integrated into the engine/UI, and passing workspace compilation.
- `PARTIALLY_SUPPORTED`: Functionality working with specific documented limitations.
- `UNSUPPORTED`: Explicitly out-of-scope or deliberately restricted (e.g., untrusted VBA macro execution).
- `PLANNED`: In architectural specification; scheduled for subsequent development.

---

## 1. Core Grid & Workbook Scale

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Sparse Cell Model (1,048,576 rows × 16,384 cols) | TESTED | `exel-cell`, `exel-core` | Navigation, Cell Edit | Bounded by host RAM; cells sparse-allocated | Win/Linux/macOS | 18 tests in `exel-cell`, 17 tests in `exel-core` |
| Multi-Sheet Management | TESTED | `exel-workbook`, `apps/rust-exel` | New (+), Rename, Switch, Delete, Context Menu | Active sheet always maintained | Win/Linux/macOS | 10 tests in `exel-workbook`, regression tests in `rust-exel` |
| Virtualized Grid Rendering | TESTED | `exel-ui` | Scroll, Pan, Zoom | Renders strictly visible viewport cells + fringe | Win/Linux/macOS | Layout & viewport clipping tests |
| Active Cell & Marquee Selection | TESTED | `exel-ui` | Click, Drag, Shift+Arrow | Single cell or rectangular range | Win/Linux/macOS | Selection bounds tests |
| Drag-to-Fill Handle | IMPLEMENTED | `exel-ui` | Mouse drag corner | Basic linear series fill | Win/Linux/macOS | Fill handle event handling |
| Row & Column Resizing | TESTED | `exel-ui` | Drag header dividers | Dynamic width/height persistence | Win/Linux/macOS | Header resize handlers |
| Row & Column Insert/Delete | TESTED | `apps/rust-exel`, `exel-commands` | Ribbon Home -> Insert/Delete, Context Menu | Coordinates shifted in sparse store | Win/Linux/macOS | Full regression tests in `rust-exel` & `exel-commands` |
| Freeze Panes | TESTED | `exel-ui`, `exel-workbook` | View -> Freeze Panes | Split row/column pinned to top/left | Win/Linux/macOS | Coordinate split tests |

---

## 2. Calculation & Formula Engine

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Formula Parsing (AST) | TESTED | `exel-formula` | Enter formula `=...` | Full Excel operator precedence, cross-sheet refs | Win/Linux/macOS | 43 tests in `exel-formula` |
| Dependency Graph (DAG) | TESTED | `exel-dependency-graph` | Automatic | Kahn's topological sort for recalc ordering | Win/Linux/macOS | 20 tests (16 unit + 4 integration) |
| Cycle Detection (`#CIRCULAR!`) | TESTED | `exel-dependency-graph` | Automatic | Tarjan SCC algorithm flags self/indirect cycles | Win/Linux/macOS | Cycle detection tests |
| Math Functions (SUM, PRODUCT, ABS, ROUND, INT, MOD, POWER, SQRT, PI) | TESTED | `exel-calc` | Ribbon / AutoSum / `=...` | Standard IEEE 754 floating-point arithmetic | Win/Linux/macOS | 37 tests (33 unit + 4 integration) |
| Statistical Functions (AVERAGE, MIN, MAX, COUNT, COUNTA) | TESTED | `exel-calc` | Formulas Tab / `=...` | Non-numeric strings ignored in numeric ranges | Win/Linux/macOS | Statistical formula test vectors |
| Logical Functions (IF, AND, OR, NOT, IFERROR) | TESTED | `exel-calc` | Formulas Tab / `=...` | Nested short-circuit evaluation | Win/Linux/macOS | Logical formula test vectors |
| Lookup Functions (VLOOKUP, INDEX, MATCH) | TESTED | `exel-calc` | Formulas Tab / `=...` | Exact match and binary search support | Win/Linux/macOS | Lookup test vectors |
| Text Functions (CONCAT, LEFT, RIGHT, MID, LEN, LOWER, UPPER, TRIM) | TESTED | `exel-calc` | Formulas Tab / `=...` | UTF-8 character boundary safe | Win/Linux/macOS | Text formula test vectors |
| Standard Error Values (`#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#N/A`, `#NUM!`, `#CIRCULAR!`) | TESTED | `exel-cell`, `exel-calc` | Display in cell | Strict error propagation across formulas | Win/Linux/macOS | Error propagation test suite |

---

## 3. Formatting, Styles & Data Validation

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Shared Style Registry (`StyleTable`) | TESTED | `exel-formatting` | Home Tab | Deduplicated styles via interning IDs | Win/Linux/macOS | 11 tests in `exel-formatting` |
| Font (Family, Size, Bold, Italic, Color) | TESTED | `exel-formatting`, `exel-ui` | Ctrl+B, Ctrl+I, Home Tab | Interfaced through egui text styles | Win/Linux/macOS | Font format builder tests |
| Cell Fills & Background Colors | TESTED | `exel-formatting`, `exel-ui` | Fill Color picker | Solid fills & RGB/Hex conversion | Win/Linux/macOS | Fill constructors tests |
| Cell Borders (Thin, Thick, Double, Dashed...) | TESTED | `exel-formatting`, `exel-ui` | Borders Dropdown | Top, bottom, left, right borders | Win/Linux/macOS | Border style tests |
| Number Formatting (Currency, Percentage, Date, General) | TESTED | `exel-formatting` | Ctrl+Shift+$, %, # | Custom format strings & Excel format codes | Win/Linux/macOS | Number format tests |
| Text Alignment (Horizontal, Vertical, Wrap, Indent) | TESTED | `exel-formatting`, `exel-ui` | Alignment buttons | Left, center, right, top, bottom, wrap | Win/Linux/macOS | Alignment builder tests |
| Data Validation Criteria | TESTED | `exel-validation` | Data Tab -> Validation | Whole number, decimal, list, date range, text length | Win/Linux/macOS | 12 tests in `exel-validation` |

---

## 4. History, Commands, Tables & Pivot

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Undo / Redo Command History | TESTED | `exel-history`, `exel-commands` | Ctrl+Z / Ctrl+Y | Bounded history stack with transaction support | Win/Linux/macOS | 10 tests in `exel-history`, 10 tests in `exel-commands` |
| Cell & Range Commands | TESTED | `exel-commands` | Edit, Clear Range, Format | Atomic rollback on error | Win/Linux/macOS | Command execution & undo tests |
| Structured Tables (`Table`) | TESTED | `exel-tables` | Ctrl+T / Insert Table | Header row, Total row, unique column names | Win/Linux/macOS | 10 tests in `exel-tables` |
| Table Structured References | TESTED | `exel-tables` | `Table1[Column]`, `[@Col]` | Exact syntax matching | Win/Linux/macOS | Structured reference tests |
| Pivot Table Aggregations | TESTED | `exel-pivot` | Insert Tab -> Pivot | Sum, Count, Average, Min, Max, row/col grand totals | Win/Linux/macOS | 14 tests in `exel-pivot` |
| Data Sorting & Quick Filter | TESTED | `exel-data` | Data Tab -> Sort / Filter | Ascending/descending sort, predicates | Win/Linux/macOS | 9 tests in `exel-data` |

---

## 5. File Formats, Clipboard & Persistence

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| XLSX Reading (calamine) | TESTED | `exel-xlsx`, `apps/rust-exel` | File -> Open (.xlsx) | Reads values, formulas, sheets, styles | Win/Linux/macOS | XLSX round-trip tests |
| XLSX Writing (rust_xlsxwriter) | TESTED | `exel-xlsx`, `apps/rust-exel` | File -> Save / Save As (.xlsx) | Preserves multi-sheet workbooks, styles, formulas | Win/Linux/macOS | 3 tests in `exel-xlsx`, regression tests in `rust-exel` |
| CSV / TSV Import & Export | TESTED | `exel-csv`, `apps/rust-exel` | File -> Open / Save (.csv, .tsv) | Auto delimiter sniffing, type inference | Win/Linux/macOS | 16 tests in `exel-csv`, regression tests in `rust-exel` |
| JSON Import & Export | TESTED | `exel-csv`, `apps/rust-exel` | File -> Open / Save (.json) | Tabular records and 2D arrays | Win/Linux/macOS | JSON round-trip tests |
| Atomic File Save (tempfile) | TESTED | `exel-io`, `apps/rust-exel` | Ctrl+S / Save | Writes to temp file before replacing target | Win/Linux/macOS | 4 tests in `exel-io`, persistence tests in `rust-exel` |
| Macro / VBA Safety | TESTED | `exel-xlsx` | File Open warning | VBA macros quarantined, never executed | Win/Linux/macOS | Macro detection test |
| Clipboard Cut / Copy / Paste | TESTED | `exel-clipboard`, `apps/rust-exel` | Ctrl+X, Ctrl+C, Ctrl+V | RFC 4180-compliant TSV format via `arboard` | Win/Linux/macOS | 7 tests in `exel-clipboard`, integration in `rust-exel` |

---

## 6. UI, Productivity, Graphics & Printing

| Feature | Status | Crate/Module | User Commands | Known Limitations | Platform Coverage | Tests |
|---|---|---|---|---|---|---|
| Excel Ribbon Tabs (10 Tabs) | TESTED | `exel-ui`, `apps/rust-exel` | Ribbon bar clicks | File, Home, Insert, Page Layout, Formulas, Data, Review, View, Developer, Help | Win/Linux/macOS | Interactive dispatch & status actions |
| Formula Bar & Name Box | TESTED | `exel-ui` | Click, F2, Type | Jump-to-cell, Commit/Cancel, multiline | Win/Linux/macOS | Formula bar sync & coordinate jumps |
| Status Bar Statistics (Average, Count, Sum) | TESTED | `exel-ui` | Selection change | Computed live over non-empty numeric cells | Win/Linux/macOS | Live statistics computation |
| Format Cells Dialog | IMPLEMENTED | `exel-ui` | Ctrl+1 / Ribbon | Number, Alignment, Font, Border, Fill tabs | Win/Linux/macOS | Dialog UI components |
| Central Keyboard Shortcuts | TESTED | `exel-shortcuts` | Ctrl/Cmd keybinds | Win, Linux, Mac modifiers; F2, F4, Ctrl+Z, Ctrl+Y, etc. | Win/Linux/macOS | 14 tests in `exel-shortcuts` |
| Charts Configuration & Extraction | TESTED | `exel-charts` | Insert Tab -> Charts | Bar, Line, Pie, Scatter configurations | Win/Linux/macOS | 7 tests in `exel-charts` |
| Page Setup & Print Preview | TESTED | `exel-print` | File -> Print Preview | Margins, orientation, paper sizes, ASCII preview | Win/Linux/macOS | 6 tests in `exel-print` |
| Batch Scriptable Automation | TESTED | `exel-automation` | Batch runner | JSON script execution (SetCell, InsertRow, Recalc, Export) | Win/Linux/macOS | 7 tests in `exel-automation` |
| Graphics Fallback & Crash Resilience | TESTED | `apps/rust-exel` | App startup | Falls back from Wgpu to Glow; writes crash.log | Win/Linux/macOS | Verified fallback logic |
| Headless Spreadsheet CLI | TESTED | `apps/rust-exel-cli` | `eval`, `convert`, `info`, `recalc` | Headless execution without window display | Win/Linux/macOS | 10 tests in `rust-exel-cli` |
