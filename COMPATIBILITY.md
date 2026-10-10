# Compatibility & File Format Fidelity - Rust EXEL

This document details the file format compatibility, data exchange fidelity, and operational boundaries between Rust EXEL and Microsoft Excel / third-party spreadsheet suites.

---

## 1. Supported File Formats

### Microsoft Excel OpenXML (.xlsx)
- **Reading**: Fully supported using pure Rust OOXML parsers (`calamine`).
  - Reads numbers, booleans, dates, strings, rich text, and empty cells.
  - Preserves formula strings (e.g., `=SUM(A1:A10)`).
  - Imports workbook sheet hierarchy, sheet names, and active sheet pointers.
  - Reads cell styling including fonts, fills, borders, alignments, and number formatting codes.
- **Writing**: Fully supported via `rust_xlsxwriter`.
  - Serializes multi-sheet workbooks with full coordinate fidelity.
  - Exports calculated formula results and formula expressions.
  - Applies cell formatting (RGB font colors, bold/italic, background fills, border styles, text alignment).
  - Configures sheet grid dimensions and custom column widths.
- **Verification**: Verified via bidirectional round-trip unit and integration tests (`exel-xlsx`, `rust-exel`).

### Comma & Tab Separated Values (.csv, .tsv)
- **Reading**: High-throughput parsing (`exel-csv`) with automatic delimiter sniffing (detects `,`, `\t`, `;`, `|`).
  - Automatic type inference parses integers, floating-point numbers, booleans (`TRUE`/`FALSE`), and formula strings (`=...`).
  - Full RFC 4180 compliance handles multi-line text cells, embedded commas, and escaped double quotes (`""`).
- **Writing**: Configurable delimiter, quoting behavior, and sheet selection.
- **Fidelity Note**: As standard in flat-text formats, cell styling and multiple sheet tabs are not represented in CSV/TSV; active sheet cell values and formulas are preserved.

### Macro-Enabled Excel Workbooks (.xlsm)
- **Security & Reading Policy**: Worksheet data and formulas are read transparently.
- **Quarantine**: VBA macro binary streams (`vbaProject.bin`) are detected, flagged to the user, and quarantined without execution.
- **Safety Guarantee**: Rust EXEL will never execute untrusted embedded script code. Users can safely save or export the workbook as clean `.xlsx`.

### JavaScript Object Notation (.json)
- **Reading & Writing**: Supports both 2D array representation (`[["Col1", "Col2"], [10, 20]]`) and tabular object arrays (`[{"Col1": 10, "Col2": 20}]`).
- **Use Case**: Streamlined interoperability with web applications, REST APIs, and data science notebooks (pandas, Polars).

---

## 2. Clipboard Interoperability

Rust EXEL features system clipboard synchronization via `exel-clipboard` and `arboard`:
- **Format**: Standard Tab-Separated Values (TSV) with RFC 4180 quote escaping.
- **External Interoperability**: Seamlessly copy/paste tabular data between Rust EXEL and:
  - Microsoft Excel (Desktop and Web)
  - Google Sheets
  - LibreOffice Calc / Apple Numbers
  - Text editors and terminals
- **Grid Mapping**: Multi-cell copied regions are pasted relative to the user's active cursor coordinate, dynamically expanding over target cells.

---

## 3. Formula Engine Compatibility

Rust EXEL implements Excel formula syntax and operator precedence:

| Category | Operators / Functions Supported |
|---|---|
| **Arithmetic Operators** | `+` (addition), `-` (subtraction), `*` (multiplication), `/` (division), `^` (exponentiation), `%` (percentage) |
| **Comparison Operators** | `=` (equal), `<>` (not equal), `<` (less than), `<=` (less or equal), `>` (greater than), `>=` (greater or equal) |
| **Text Operator** | `&` (string concatenation) |
| **Math Functions** | `SUM`, `PRODUCT`, `ABS`, `ROUND`, `INT`, `MOD`, `POWER`, `SQRT`, `PI` |
| **Statistical Functions** | `AVERAGE`, `MIN`, `MAX`, `COUNT`, `COUNTA` |
| **Logical Functions** | `IF`, `AND`, `OR`, `NOT`, `IFERROR` |
| **Lookup Functions** | `VLOOKUP`, `INDEX`, `MATCH` |
| **Text Functions** | `CONCAT`, `LEFT`, `RIGHT`, `MID`, `LEN`, `LOWER`, `UPPER`, `TRIM` |
| **Reference Formats** | Relative (`A1`), Absolute (`$A$1`), Mixed (`$A1`, `A$1`), Ranges (`A1:B10`), Cross-Sheet (`Sheet2!A1`, `'Quarterly Sales'!B5:C20`) |
| **Error Propagation** | `#DIV/0!`, `#VALUE!`, `#REF!`, `#NAME?`, `#N/A`, `#NUM!`, `#CIRCULAR!` |

---

## 4. Unsupported & Restricted Features

To maintain transparency, the following features are explicitly unsupported or deliberately constrained:

1. **VBA / Macro Execution**: Restricted by design. Rust EXEL does not include a VBA or VBScript runtime. Files with macros open in safe read-only or clean-export mode.
2. **Legacy Binary Excel (.xls / BIFF8)**: Legacy binary formats (Excel 97–2003) are not natively parsed. Users should convert `.xls` files to `.xlsx` prior to opening.
3. **External OLE Links / DDE**: Dynamic Data Exchange and Object Linking and Embedding linking to external Windows processes are not executed.
4. **ActiveX Controls**: Interactive ActiveX form controls embedded in legacy sheets are ignored.
