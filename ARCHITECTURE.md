# Rust EXEL - System Architecture

Rust EXEL is an independent, native, cross-platform spreadsheet application written in pure Rust.

## Architectural Principles
1. **Decoupled Engine**: The core workbook and formula calculation engine are completely decoupled from the graphical user interface.
2. **Sparse Matrix Storage**: Worksheets support standard Excel limits (1,048,576 rows by 16,384 columns, A..XFD) through sparse key-indexed structures without preallocating empty cells.
3. **Hardware-Accelerated Virtualization**: The UI utilizes `egui` and `eframe` with `wgpu` to render only visible viewport cells plus a small prefetch fringe, keeping frame rates at a steady 60+ FPS regardless of workbook size.
4. **Adapter Pattern for Formula & I/O**: Pluggable calculation adapters and I/O handlers ensure independence from individual third-party libraries while guaranteeing Excel formula semantics.
5. **Deterministic Command Pattern**: Every mutation to the workbook (cell edits, structural changes, formatting) executes through an undoable command on an undo/redo stack.

## Workspace Layout
```
rust-exel/
├── crates/
│   ├── exel-core/              # Fundamental types: CellCoord, CellRange, SheetId, CellAddress
│   ├── exel-cell/              # Cell model: CellValue, CellError, CellState, sparse storage
│   ├── exel-workbook/          # Workbook, Worksheet, sheet collections, bounds
│   ├── exel-dependency-graph/  # Directed graph tracking formula dependencies and cycle detection
│   ├── exel-formula/           # AST parser, lexer, reference resolver, formula validator
│   ├── exel-calc/              # Formula evaluator, function registry, calculation coordinator
│   ├── exel-formatting/        # Fonts, fills, borders, alignments, number format strings
│   ├── exel-tables/            # Structured tables, table styles, headers, totals
│   ├── exel-validation/        # Data validation rules, dropdowns, input alerts
│   ├── exel-history/           # Undo/Redo history stack manager
│   ├── exel-commands/          # Concrete undoable commands (SetCell, InsertRow, etc.)
│   ├── exel-clipboard/         # System clipboard TSV/CSV serialization & deserialization
│   ├── exel-io/                # File persistence traits, atomic save with tempfile
│   ├── exel-csv/               # CSV, TSV, and JSON importers/exporters
│   ├── exel-xlsx/              # High-fidelity XLSX read/write adapter (calamine & rust_xlsxwriter)
│   ├── exel-charts/            # Chart data extraction and visual configuration
│   ├── exel-pivot/             # Pivot aggregation engine and multi-dimensional summaries
│   ├── exel-data/              # Analysis tools: statistics, regression, transforms
│   ├── exel-print/             # Page setup, margin calculations, PDF generation
│   ├── exel-shortcuts/         # Central ShortcutRegistry (Windows, Linux, macOS)
│   ├── exel-ui/                # Virtualized grid widget, Ribbon, Formula Bar, dialogs
│   └── exel-automation/        # Future sandboxed macro scripting interface
└── apps/
    ├── rust-exel/              # Native desktop GUI binary
    └── rust-exel-cli/          # Headless command-line spreadsheet calculation tool
```

## Calculation Pipeline
1. Cell edit commits new value/formula.
2. `exel-dependency-graph` analyzes AST references and rebuilds directed edges.
3. Cycle detection verifies absence of circular references; circular nodes are tagged with `#CIRCULAR!`.
4. Downstream topological ordering isolates strictly dependent cells.
5. Dependent cells are evaluated in order and cache their new values.
6. The virtualized grid displays formatted cached values without re-evaluating formulas during rendering.
