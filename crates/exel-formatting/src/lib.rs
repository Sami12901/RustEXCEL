pub mod alignment;
pub mod border;
pub mod color;
pub mod fill;
pub mod font;
pub mod number_format;
pub mod style;
pub mod table;

pub use alignment::{Alignment, HorizontalAlignment, VerticalAlignment};
pub use border::{Border, BorderSide, BorderStyle};
pub use color::Color;
pub use fill::{Fill, FillPattern};
pub use font::Font;
pub use number_format::{
    datetime_to_excel_serial, excel_serial_to_datetime, DateFormat, DateTimeFormat,
    NegativeNumberFormat, NumberFormat, TimeFormat,
};
pub use style::CellStyle;
pub use table::StyleTable;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatting_integration() {
        let mut table = StyleTable::new();
        let currency_style = CellStyle::default()
            .with_font(Font::new("Segoe UI", 12.0).with_bold(true))
            .with_fill(Fill::solid(Color::rgb(240, 240, 240)))
            .with_border(Border::bottom_only(BorderStyle::Thin, Color::BLACK))
            .with_alignment(Alignment::new().with_horizontal(HorizontalAlignment::Right))
            .with_number_format(NumberFormat::currency("$", 2));

        let id = table.get_or_insert(currency_style);
        assert_eq!(id, 1);

        let retrieved = table.get(id).unwrap();
        assert_eq!(retrieved.font.name, "Segoe UI");
        assert_eq!(retrieved.number_format.format_number(1234.5), "$1,234.50");
    }
}
