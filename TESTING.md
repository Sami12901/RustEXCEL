# Testing Strategy - Rust EXEL

## Quality & Verification Philosophy
- **Formula Accuracy**: Mathematical and logical functions are tested against verified spreadsheet evaluation vectors.
- **Round-Trip Integrity**: Saving to XLSX, CSV, or TSV and reopening must preserve cell coordinates, raw values, formulas, and formatting without degradation.
- **Memory Safety & Sparse Scaling**: Worksheets must be tested with cells at the extreme boundaries (Row 1,048,576 and Column 16,384 `XFD`) to guarantee sparse efficiency without memory exhaustion.

## Running Tests
```bash
# Run all unit and integration tests across the workspace
cargo test --workspace

# Run tests for a specific crate
cargo test -p exel-calc
cargo test -p exel-xlsx

# Run benchmarks
cargo bench
```
