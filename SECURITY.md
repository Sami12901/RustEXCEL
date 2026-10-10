# Security Policy - Rust EXEL

## Untrusted Workbook Handling
Spreadsheets often originate from external or untrusted sources. Rust EXEL adheres to strict security defaults:

1. **No Automatic Macro Execution**: Rust EXEL never executes VBA macros or embedded executable code found in XLSM or XLSX files.
2. **Safe ZIP & OOXML Decompression**: Workbook packages are decompressed using strict size quotas to defend against zip bombs and XML entity expansion (Billion Laughs) attacks.
3. **Atomic File Persistence**: When saving over an existing workbook file, Rust EXEL writes to a temporary file (`tempfile`) first and atomically replaces the destination. A failure during saving will never corrupt or truncate the existing file.
4. **Sandboxed Formula Evaluation**: Formulas are evaluated purely within an in-memory Rust AST and cannot access filesystem, process, or network resources.

## Reporting a Vulnerability
To report a security vulnerability, please submit a private advisory through the GitHub repository or contact security@rust-exel.org.
