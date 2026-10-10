use crate::error::ClipboardError;
use crate::range::{cells_to_tsv, paste_tsv_to_sheet};
use arboard::Clipboard;
use exel_cell::{CellCoord, CellRange};
use exel_workbook::Worksheet;

/// Safely copies text to the operating system clipboard using `arboard`.
pub fn copy_text_to_system_clipboard(text: &str) -> Result<(), ClipboardError> {
    let mut cb = Clipboard::new().map_err(|e| ClipboardError::System(e.to_string()))?;
    cb.set_text(text)
        .map_err(|e| ClipboardError::System(e.to_string()))?;
    Ok(())
}

/// Safely retrieves text from the operating system clipboard using `arboard`.
pub fn get_text_from_system_clipboard() -> Result<String, ClipboardError> {
    let mut cb = Clipboard::new().map_err(|e| ClipboardError::System(e.to_string()))?;
    let text = cb
        .get_text()
        .map_err(|e| ClipboardError::System(e.to_string()))?;
    Ok(text)
}

/// High-level function: copies a range from the active worksheet to system clipboard as TSV.
pub fn copy_range(sheet: &Worksheet, range: CellRange) -> Result<(), ClipboardError> {
    let tsv = cells_to_tsv(sheet, range);
    copy_text_to_system_clipboard(&tsv)
}

/// High-level function: pastes system clipboard content into the worksheet starting at `anchor`.
pub fn paste_to_sheet(
    sheet: &mut Worksheet,
    anchor: CellCoord,
) -> Result<Vec<CellCoord>, ClipboardError> {
    let text = get_text_from_system_clipboard()?;
    Ok(paste_tsv_to_sheet(sheet, anchor, &text))
}
