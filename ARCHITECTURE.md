# Rust EXEL - System Architecture

Rust EXEL is an independent, native, cross-platform spreadsheet application and computational engine engineered in pure Rust.

---

## 1. Core Architectural Principles

1. **Decoupled Engine & GUI**: The core workbook, sparse cell store, formula parser, dependency graph, and calculation engine are completely decoupled from the graphical user interface. Every engine crate compiles independently and can run in headless CLI or embedded server environments.
2. **Sparse Key-Indexed Matrix Storage**: Worksheets support standard Excel scale (**1,048,576 rows × 16,384 columns / A..XFD**) through `BTreeMap<CellCoord, CellState>` storage in `exel-cell`, avoiding preallocation of empty cells and maintaining a minimal memory footprint.
3. **Hardware-Accelerated Virtualized Grid**: The UI (`exel-ui`) uses `egui` and `eframe` with `wgpu` to render only the visible viewport cells plus a small prefetch margin, guaranteeing a steady 60+ FPS navigation even across millions of rows.
4. **Resilient Graphics Pipeline**: In `apps/rust-exel`, the application implements an automatic fallback mechanism: if `wgpu` graphics initialization fails (e.g., outdated graphics drivers, headless RDP, or VM environments), the application gracefully falls back to OpenGL via `glow`, logging diagnostics to `crash.log`.
5. **Topological Calculation DAG**: Formula dependencies are tracked via a Directed Acyclic Graph (`exel-dependency-graph`). Kahn's topological sort determines minimal evaluation order for dirty cells, and Tarjan's Strongly Connected Components algorithm detects circular dependency cycles, isolating them with `#CIRCULAR!`.
6. **Deterministic Undo/Redo Commands**: Mutations (cell edits, range clearing, formatting, row/column structural insertion/deletion) are encapsulated as commands in `exel-commands` executing against a bounded transaction history in `exel-history`.
7. **Atomic, Crash-Proof I/O**: File persistence writes first to a temporary file via `tempfile` and atomically replaces the destination only upon successful serialization, preventing corruption during power failure or process interruptions.

---

## 2. Workspace Layout

```
rust-exel/
├── apps/
│   ├── rust-exel/              # Native desktop GUI application (eframe + wgpu / glow fallback)
│   └── rust-exel-cli/          # Headless CLI for formula evaluation, batch conversion & inspection
├── crates/
│   ├── exel-core/              # Coordinate primitives: CellCoord, CellRange, SheetId, A1 conversions
│   ├── exel-cell/              # SparseCellStore, CellValue, CellState, CellError models
│   ├── exel-workbook/          # Multi-sheet Workbook, Worksheet, dimensions, named ranges
│   ├── exel-dependency-graph/  # Directed graph tracking formula dependencies, dirty state, cycle detection
│   ├── exel-formula/           # AST parser, lexer, reference resolver, formula validator
│   ├── exel-calc/              # Formula evaluator, function registry, recalculation engine
│   ├── exel-formatting/        # Fonts, fills, borders, alignments, number format strings, StyleTable
│   ├── exel-commands/          # Concrete undoable commands (SetCell, InsertRow, DeleteCol, FormatRange)
│   ├── exel-history/           # Undo/Redo bounded stack manager with transactional semantics
│   ├── exel-tables/            # Structured tables, table styles, headers, totals, structured references
│   ├── exel-validation/        # Data validation criteria (integers, decimals, lists, date ranges, text length)
│   ├── exel-clipboard/         # RFC-compliant TSV clipboard formatting, parsing & arboard system clipboard
│   ├── exel-io/                # File persistence traits, format sniffing, atomic save with tempfile
│   ├── exel-csv/               # CSV, TSV, and JSON importers and exporters with delimiter sniffing
│   ├── exel-xlsx/              # High-fidelity XLSX read/write adapter (calamine & rust_xlsxwriter)
│   ├── exel-charts/            # Chart configurations, series extraction, visual properties
│   ├── exel-pivot/             # Pivot aggregation engine and multi-dimensional summaries
│   ├── exel-data/              # Data analysis tools: descriptive statistics, regression, sorting, quick filters
│   ├── exel-print/             # Page setup, paper sizes, margin calculations, ASCII/PDF print preview
│   ├── exel-shortcuts/         # Central cross-platform ShortcutRegistry (Windows, Linux, macOS)
│   ├── exel-automation/        # Batch scriptable automation runner (JSON scripts)
│   └── exel-ui/                # Virtualized grid widget, Ribbon (10 tabs), Formula Bar, Sheet tabs, dialogs
└── assets/
    └── icons/                  # High-resolution application logos and brand icons
```

---

## 3. Data Flow & Subsystem Interaction

### A. Calculation Pipeline
1. **User Edit**: User modifies a cell in the GUI grid or formula bar.
2. **AST Parsing**: `exel-formula` tokenizes and parses the input into an Abstract Syntax Tree (AST).
3. **Graph Dependency Update**: `exel-dependency-graph` analyzes AST references (including cross-sheet references) and updates upstream/downstream edges.
4. **Cycle Detection**: The graph evaluates cycles; recursive references receive `#CIRCULAR!`.
5. **Topological Evaluation**: Dirty dependent cells are topologically sorted and evaluated in order by `exel-calc::CalcEngine`.
6. **Cached Value Presentation**: The virtualized grid displays formatted cached values without recalculating during 60 FPS repaint loops.

### B. GUI Event & Ribbon Dispatch
1. **Ribbon UI**: `exel-ui::Ribbon` renders 10 functional tabs (*File, Home, Insert, Page Layout, Formulas, Data, Review, View, Developer, Help*).
2. **Action Emission**: Clicking a button emits a structured `RibbonAction` (e.g., `NewWorkbook`, `OpenFile`, `SaveFile`, `Cut`, `Copy`, `Paste`, `InsertRow`, `DeleteColumn`, `Recalculate`, `StatusMessage`).
3. **App Handling**: `apps/rust-exel` matches the action, executes workbook operations or file dialogs, and pushes user feedback to the bottom status bar notification system.

### C. Clipboard Data Exchange
1. **Copy / Cut**: Selected range cells are serialized into RFC 4180-compliant tab-separated values (TSV) with proper quote escaping by `exel-clipboard`. The TSV text is synced to the OS clipboard via `arboard`.
2. **Paste**: Clipboard text is fetched and parsed into a 2D matrix of cells, mapped starting from the active cell coordinate, and committed to the worksheet.

### D. File I/O & Format Detection
1. **Extension & Content Sniffing**: Path extensions (`.xlsx`, `.csv`, `.tsv`, `.json`) determine the codec.
2. **Serialization**: Workbook data is serialized via `exel-xlsx`, `exel-csv`, or `exel-io`.
3. **Atomic Commit**: Data is written to a temporary sibling file and renamed onto the target file, guaranteeing file integrity.
