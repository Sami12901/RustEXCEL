# Compatibility & File Format Fidelity - Rust EXEL

## Supported Formats

### 1. Microsoft Excel Workbook (.xlsx)
- **Reading**: Fully supported via pure Rust OOXML parsers. Imports cell values, types (numbers, dates, text, booleans), formulas, merged cells, sheet names, and dimensions.
- **Writing**: Fully supported via `rust_xlsxwriter`. Exports full multi-sheet workbooks with values, formulas, number formatting, colors, and borders.
- **Round-Trip Testing**: Continuous integration runs bidirectional verification testing on sample workbooks.

### 2. Comma / Tab Separated Values (.csv, .tsv)
- **Reading & Writing**: Fully supported. Preserves row/column coordinates, cell values, and strings. Number formatting and colors are intentionally discarded with user notification as per standard CSV behavior.

### 3. Excel Macro-Enabled Workbook (.xlsm)
- **Reading**: Values and formulas are read. VBA macro streams are detected; users are notified that macros are not executed.
- **Writing**: Can be safely exported as standard `.xlsx`.

### 4. JavaScript Object Notation (.json)
- **Reading & Writing**: Supports tabular and structured array import and export for data science pipelines.
