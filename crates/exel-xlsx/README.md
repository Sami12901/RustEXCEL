# exel-xlsx

High-performance Microsoft Excel XLSX file reader, writer, and macro security analyzer for the Rust EXEL spreadsheet engine.

## Architectural Overview

`exel-xlsx` bridges the internal in-memory document structures (`Workbook`, `Worksheet`, `SparseCellStore`, `CellStyle`, `StyleTable`) with the OpenXML spreadsheet package format (.xlsx, .xlsm).

- **Read Pipeline**: Powered by `calamine`. Reads all worksheets, cell coordinates, data types, and cached formulas via low-level OpenXML relationships.
- **Write Pipeline**: Powered by `rust_xlsxwriter`. Supports complete style translation (fonts, borders, fills, alignments, number formats), dimensions (column widths, row heights), freeze panes, tab colors, and formulas.
- **Security & Macro Detection**: Inspects ZIP directory records for VBA project payloads (`xl/vbaProject.bin`, `xl/vbaProjectSignature.bin`, macrosheets) to safely isolate active script content before evaluation.

---

## Read Capabilities & Limitations

### Capabilities
- **Multi-Sheet Workbooks**: Automatically reads all worksheets into discrete `Worksheet` instances with original sheet names.
- **Data Types**:
  - Empty cells (`Data::Empty` -> `CellValue::Empty`)
  - Strings (`Data::String` -> `CellValue::Text`)
  - Numbers (`Data::Float`, `Data::Int` -> `CellValue::Number(f64)`)
  - Booleans (`Data::Bool` -> `CellValue::Boolean`)
  - Date & DateTime (`Data::DateTime`, `Data::DateTimeIso` -> `CellValue::DateTime` mapped via Excel serial epoch)
  - Error Codes (`#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#NUM!`, `#NULL!`, `#N/A!`)
- **Formula Ingestion**: Extracts formula definitions from OpenXML cell formula definitions.
- **Sparse Cell Population**: Populates only non-empty cells into `SparseCellStore`, avoiding multi-gigabyte memory footprints for distant sparse coordinates.
- **Macro Scanning**: Detects VBA and XL4 macros without executing any foreign code.

### Limitations
- **Pivot Tables & External Connections**: Reading pivot cache definitions and external OLE DB/ODBC queries is not yet parsed into interactive pivot models.
- **Embedded Drawing Objects & Shapes**: Shape coordinates and raster pictures inside worksheets are ignored during ingestion.
- **VBA Execution**: VBA code cannot and will not be executed. Files with VBA macros generate security warnings.

---

## Write Capabilities & Limitations

### Capabilities
- **Multi-Sheet Serialization**: Preserves sheet order and names.
- **Cell Styling & Formatting**:
  - Fonts (name, size, bold, italic, underline, strikethrough, RGB colors)
  - Fills (solid foreground RGB background fills)
  - Borders (thin, medium, thick, double, dashed, RGB colors on top, bottom, left, right)
  - Alignment (horizontal: left, center, right, justify; vertical: top, center, bottom; wrap text)
  - Number Formats (standard formats, custom currencies `$#,##0.00`, percentages `0.0%`, dates `yyyy-mm-dd`)
- **Sheet Properties**:
  - Custom column widths
  - Custom row heights
  - Freeze panes (horizontal and vertical split coordinates)
  - Sheet tab colors (RGB)
- **Formulas**: Writes standard formula expressions (e.g. `=SUM(A1:B10)`) with target cell formatting.
- **Atomic File Saving**: Uses temporary file replacement (`save_atomic`) to prevent data corruption during disk writes.

### Limitations
- **Password-Protected / Encrypted XLSX**: Encrypted XLSX containers (Agile Encryption) are not supported.
- **Data Validation & Conditional Formatting XML**: Advanced conditional formatting rules are evaluated at the UI layer; raw conditional formatting rule XML tags are not serialized to XLSX.
