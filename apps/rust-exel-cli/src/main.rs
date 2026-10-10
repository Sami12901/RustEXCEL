mod calc;
mod converter;

use anyhow::{bail, Result};
use calc::{evaluate_formula, recalculate_workbook};
use converter::{read_csv, read_xlsx, write_csv, write_xlsx};
use exel_cell::CellCoord;
use exel_workbook::Workbook;
use std::env;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "recalculate" | "recalc" => {
            if args.len() < 3 {
                eprintln!("Usage: rust-exel-cli recalculate <file> [-o <output_file>]");
                std::process::exit(1);
            }
            let input_path = PathBuf::from(&args[2]);
            let output_path = if args.len() >= 5 && args[3] == "-o" {
                Some(PathBuf::from(&args[4]))
            } else if args.len() >= 4 && !args[3].starts_with('-') {
                Some(PathBuf::from(&args[3]))
            } else {
                None
            };

            cmd_recalculate(&input_path, output_path)?;
        }
        "convert" => {
            if args.len() < 4 {
                eprintln!("Usage: rust-exel-cli convert <input_file> <output_file>");
                std::process::exit(1);
            }
            let input_path = PathBuf::from(&args[2]);
            let output_path = PathBuf::from(&args[3]);
            cmd_convert(&input_path, &output_path)?;
        }
        "info" | "inspect" => {
            if args.len() < 3 {
                eprintln!("Usage: rust-exel-cli info <file>");
                std::process::exit(1);
            }
            let input_path = PathBuf::from(&args[2]);
            cmd_info(&input_path)?;
        }
        "eval" => {
            if args.len() < 3 {
                eprintln!("Usage: rust-exel-cli eval \"<formula>\"");
                std::process::exit(1);
            }
            let formula = &args[2];
            cmd_eval(formula)?;
        }
        "--help" | "-h" | "help" => {
            print_help();
        }
        other => {
            eprintln!("Unknown command: '{}'", other);
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_help() {
    println!("Rust EXCEL CLI - Headless Spreadsheet Processor");
    println!("Usage:");
    println!("  rust-exel-cli recalculate <file> [-o <output>]  Recalculate all formulas in a workbook or CSV");
    println!("  rust-exel-cli convert <input> <output>         Convert between XLSX, CSV, and formats");
    println!("  rust-exel-cli info <file>                      Inspect sheet dimensions and statistics");
    println!("  rust-exel-cli eval \"<formula>\"                  Evaluate a formula directly (e.g. \"=SUM(1, 2, 3)\")");
    println!("  rust-exel-cli --help                           Show this help message");
}

fn load_workbook(path: &Path) -> Result<Workbook> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "csv" => read_csv(path),
        "xlsx" | "xlsm" | "xls" => read_xlsx(path),
        _ => {
            // Try CSV first, then XLSX
            if let Ok(wb) = read_csv(path) {
                Ok(wb)
            } else {
                read_xlsx(path)
            }
        }
    }
}

fn cmd_recalculate(input: &Path, output: Option<PathBuf>) -> Result<()> {
    println!("Loading workbook from '{}'...", input.display());
    let mut wb = load_workbook(input)?;

    println!("Recalculating formulas...");
    recalculate_workbook(&mut wb);

    if let Some(out_path) = output {
        println!("Saving recalculated workbook to '{}'...", out_path.display());
        let ext = out_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if ext == "csv" {
            write_csv(wb.active_sheet(), &out_path)?;
        } else {
            write_xlsx(&wb, &out_path)?;
        }
        println!("Successfully written to '{}'!", out_path.display());
    } else {
        // Print ASCII summary of active sheet
        println!("\nRecalculation Complete! Active Sheet Preview ({}):", wb.active_sheet().name);
        print_sheet_ascii(wb.active_sheet());
    }

    Ok(())
}

fn cmd_convert(input: &Path, output: &Path) -> Result<()> {
    println!("Converting '{}' -> '{}'...", input.display(), output.display());
    let wb = load_workbook(input)?;

    let out_ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match out_ext.as_str() {
        "csv" => {
            write_csv(wb.active_sheet(), output)?;
        }
        "xlsx" => {
            write_xlsx(&wb, output)?;
        }
        other => {
            bail!("Unsupported output format: '.{}'. Supported formats: .csv, .xlsx", other);
        }
    }

    println!("Conversion successful!");
    Ok(())
}

fn cmd_info(input: &Path) -> Result<()> {
    let wb = load_workbook(input)?;

    println!("File: {}", input.display());
    println!("Worksheet count: {}", wb.sheet_count());
    println!("--------------------------------------------------");

    for (idx, sheet) in wb.sheets.iter().enumerate() {
        println!("Sheet #{} : \"{}\"", idx + 1, sheet.name);
        let populated_cells = sheet.cells.len();
        let formula_count = sheet.cells.iter().filter(|(_, s)| s.formula.is_some()).count();

        if let Some(range) = sheet.cells.used_range() {
            println!("  Used Range: {} ({} rows x {} columns)", range.to_a1(), range.row_count(), range.col_count());
        } else {
            println!("  Used Range: Empty");
        }
        println!("  Populated Cells: {}", populated_cells);
        println!("  Formula Cells:   {}", formula_count);
        println!();
    }

    Ok(())
}

fn cmd_eval(formula: &str) -> Result<()> {
    let dummy_sheet = exel_workbook::Worksheet::new(exel_core::SheetId::new("calc"), "Calc");
    let result = evaluate_formula(&dummy_sheet, formula);
    println!("{} = {}", formula, result);
    Ok(())
}

fn print_sheet_ascii(sheet: &exel_workbook::Worksheet) {
    if let Some(range) = sheet.cells.used_range() {
        let max_r = range.end.row.min(25);
        let max_c = range.end.col.min(10);

        println!("+------+----------------------------------------------------------+");
        for r in 1..=max_r {
            print!("| R{:02} |", r);
            for c in 1..=max_c {
                if let Ok(coord) = CellCoord::new(r, c) {
                    let text = sheet.get_cell_value(coord).to_display_string();
                    print!(" {:^10} |", text);
                }
            }
            println!();
        }
        println!("+------+----------------------------------------------------------+");
        if range.end.row > max_r || range.end.col > max_c {
            println!("(Showing first {} rows and {} cols out of {} rows x {} cols)", max_r, max_c, range.row_count(), range.col_count());
        }
    } else {
        println!("(Sheet is empty)");
    }
}
