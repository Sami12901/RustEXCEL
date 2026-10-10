use exel_cell::CellValue;

/// Escapes a single string field for TSV format.
/// If the text contains double-quotes, tabs, or newlines, it is enclosed
/// in double-quotes and existing double-quotes are doubled ("").
pub fn escape_tsv_cell(text: &str) -> String {
    if text.contains('"') || text.contains('\t') || text.contains('\n') || text.contains('\r') {
        let escaped = text.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        text.to_string()
    }
}

/// Formats a 2D rectangular grid of string slices into a TSV string with standard CRLF line endings.
pub fn format_tsv<T: AsRef<str>>(grid: &[Vec<T>]) -> String {
    let mut out = String::new();
    for row in grid {
        let row_str = row
            .iter()
            .map(|cell| escape_tsv_cell(cell.as_ref()))
            .collect::<Vec<_>>()
            .join("\t");
        out.push_str(&row_str);
        out.push_str("\r\n");
    }
    out
}

/// Formats a 2D rectangular grid of `CellValue`s into a TSV string.
pub fn format_tsv_values(grid: &[Vec<CellValue>]) -> String {
    let mut str_grid: Vec<Vec<String>> = Vec::with_capacity(grid.len());
    for row in grid {
        let mut str_row = Vec::with_capacity(row.len());
        for cell in row {
            str_row.push(cell.to_display_string());
        }
        str_grid.push(str_row);
    }
    format_tsv(&str_grid)
}

/// Parses a TSV text into a 2D grid of Strings, correctly respecting quotes,
/// escaped quotes (`""`), embedded tabs (`\t`), and embedded newlines (`\r\n` or `\n`).
pub fn parse_tsv(input: &str) -> Vec<Vec<String>> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_field = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;
    let mut field_started = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    // Escaped double-quote `""` inside quotes
                    chars.next();
                    current_field.push('"');
                } else {
                    // Closing quote
                    in_quotes = false;
                }
            } else {
                current_field.push(ch);
            }
        } else {
            match ch {
                '"' if !field_started => {
                    in_quotes = true;
                    field_started = true;
                }
                '\t' => {
                    current_row.push(current_field);
                    current_field = String::new();
                    field_started = false;
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    current_row.push(current_field);
                    current_field = String::new();
                    rows.push(current_row);
                    current_row = Vec::new();
                    field_started = false;
                }
                '\n' => {
                    current_row.push(current_field);
                    current_field = String::new();
                    rows.push(current_row);
                    current_row = Vec::new();
                    field_started = false;
                }
                _ => {
                    field_started = true;
                    current_field.push(ch);
                }
            }
        }
    }

    // Append last field and row if any uncommitted tokens remain
    if field_started || in_quotes || !current_field.is_empty() || !current_row.is_empty() {
        current_row.push(current_field);
        rows.push(current_row);
    }

    // If input ends with newline, standard parsing may produce a trailing single-element row `[""]`.
    // We trim this trailing empty row to match standard spreadsheet clipboard semantics.
    if rows.len() > 1 && rows.last().is_some_and(|r| r.len() == 1 && r[0].is_empty()) {
        rows.pop();
    }

    rows
}

/// Parses TSV text into a 2D grid of `CellValue` items, parsing numbers, booleans, and empty cells.
pub fn parse_tsv_values(input: &str) -> Vec<Vec<CellValue>> {
    let raw = parse_tsv(input);
    raw.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|field| {
                    let trimmed = field.trim();
                    if trimmed.is_empty() {
                        CellValue::Empty
                    } else if let Ok(n) = trimmed.parse::<f64>() {
                        CellValue::Number(n)
                    } else if trimmed.eq_ignore_ascii_case("true") {
                        CellValue::Boolean(true)
                    } else if trimmed.eq_ignore_ascii_case("false") {
                        CellValue::Boolean(false)
                    } else {
                        CellValue::Text(field)
                    }
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsv_basic_roundtrip() {
        let grid = vec![
            vec!["Name".to_string(), "Score".to_string()],
            vec!["Alice".to_string(), "95".to_string()],
            vec!["Bob".to_string(), "88.5".to_string()],
        ];

        let formatted = format_tsv(&grid);
        let parsed = parse_tsv(&formatted);
        assert_eq!(grid, parsed);
    }

    #[test]
    fn test_tsv_quoted_fields_with_tabs_quotes_newlines() {
        let grid = vec![
            vec!["Hello\tWorld".to_string(), "Quote: \"Special\"".to_string()],
            vec!["Line 1\nLine 2".to_string(), "Simple".to_string()],
        ];

        let formatted = format_tsv(&grid);
        assert!(formatted.contains("\"Hello\tWorld\""));
        assert!(formatted.contains("\"Quote: \"\"Special\"\"\""));
        assert!(formatted.contains("\"Line 1\nLine 2\""));

        let parsed = parse_tsv(&formatted);
        assert_eq!(grid, parsed);
    }

    #[test]
    fn test_tsv_parse_values() {
        let tsv = "100\tTRUE\tHello\r\n200.5\tFALSE\t\r\n";
        let values = parse_tsv_values(tsv);

        assert_eq!(values.len(), 2);
        assert_eq!(values[0][0], CellValue::Number(100.0));
        assert_eq!(values[0][1], CellValue::Boolean(true));
        assert_eq!(values[0][2], CellValue::Text("Hello".to_string()));

        assert_eq!(values[1][0], CellValue::Number(200.5));
        assert_eq!(values[1][1], CellValue::Boolean(false));
        assert_eq!(values[1][2], CellValue::Empty);
    }

    #[test]
    fn test_tsv_empty_string() {
        let parsed = parse_tsv("");
        assert!(parsed.is_empty());

        let values = parse_tsv_values("");
        assert!(values.is_empty());
    }
}
