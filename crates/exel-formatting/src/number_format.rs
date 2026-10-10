use chrono::{NaiveDate, NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum NegativeNumberFormat {
    #[default]
    Minus,        // -123.45
    Parentheses,  // (123.45)
    RedMinus,     // [Red]-123.45
    RedParentheses, // [Red](123.45)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DateFormat {
    #[default]
    YyyyMmDd,      // yyyy-mm-dd
    MmDdYyyy,      // mm/dd/yyyy
    DdMmYyyy,      // dd/mm/yyyy
    YyyyMmDdSlash, // yyyy/mm/dd
    MmmDdYyyy,     // mmm dd, yyyy
    DdMmmYyyy,     // dd-mmm-yyyy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TimeFormat {
    #[default]
    HhMmSs24,      // hh:mm:ss
    HhMm24,        // hh:mm
    HhMmSs12,      // hh:mm:ss AM/PM
    HhMm12,        // hh:mm AM/PM
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DateTimeFormat {
    #[default]
    Iso8601,       // yyyy-mm-dd hh:mm:ss
    UsStandard,    // mm/dd/yyyy hh:mm:ss
}

/// Rich NumberFormat specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NumberFormat {
    General,
    Number {
        decimal_places: u8,
        use_thousands_separator: bool,
        negative_format: NegativeNumberFormat,
    },
    Currency {
        symbol: String,
        decimal_places: u8,
        negative_format: NegativeNumberFormat,
    },
    Percentage {
        decimal_places: u8,
    },
    Date(DateFormat),
    Time(TimeFormat),
    DateTime(DateTimeFormat),
    Text,
    Custom(String),
}

impl Default for NumberFormat {
    fn default() -> Self {
        Self::General
    }
}

impl NumberFormat {
    pub fn number(decimal_places: u8, use_thousands_separator: bool) -> Self {
        Self::Number {
            decimal_places,
            use_thousands_separator,
            negative_format: NegativeNumberFormat::Minus,
        }
    }

    pub fn currency<S: Into<String>>(symbol: S, decimal_places: u8) -> Self {
        Self::Currency {
            symbol: symbol.into(),
            decimal_places,
            negative_format: NegativeNumberFormat::Minus,
        }
    }

    pub fn percentage(decimal_places: u8) -> Self {
        Self::Percentage { decimal_places }
    }

    pub fn to_format_string(&self) -> String {
        match self {
            NumberFormat::General => "General".to_string(),
            NumberFormat::Number {
                decimal_places,
                use_thousands_separator,
                negative_format,
            } => {
                let base = if *use_thousands_separator {
                    if *decimal_places > 0 {
                        format!("#,##0.{}", "0".repeat(*decimal_places as usize))
                    } else {
                        "#,##0".to_string()
                    }
                } else if *decimal_places > 0 {
                    format!("0.{}", "0".repeat(*decimal_places as usize))
                } else {
                    "0".to_string()
                };

                match negative_format {
                    NegativeNumberFormat::Minus => base,
                    NegativeNumberFormat::Parentheses => format!("{};({})", base, base),
                    NegativeNumberFormat::RedMinus => format!("{};[Red]-{}", base, base),
                    NegativeNumberFormat::RedParentheses => format!("{};[Red]({})", base, base),
                }
            }
            NumberFormat::Currency {
                symbol,
                decimal_places,
                negative_format,
            } => {
                let dec = if *decimal_places > 0 {
                    format!(".{}", "0".repeat(*decimal_places as usize))
                } else {
                    String::new()
                };
                let pos = format!("{symbol}#,##0{dec}");
                match negative_format {
                    NegativeNumberFormat::Minus => format!("{pos};-{pos}"),
                    NegativeNumberFormat::Parentheses => format!("{pos};({pos})"),
                    NegativeNumberFormat::RedMinus => format!("{pos};[Red]-{pos}"),
                    NegativeNumberFormat::RedParentheses => format!("{pos};[Red]({pos})"),
                }
            }
            NumberFormat::Percentage { decimal_places } => {
                if *decimal_places > 0 {
                    format!("0.{}%", "0".repeat(*decimal_places as usize))
                } else {
                    "0%".to_string()
                }
            }
            NumberFormat::Date(df) => match df {
                DateFormat::YyyyMmDd => "yyyy-mm-dd".to_string(),
                DateFormat::MmDdYyyy => "mm/dd/yyyy".to_string(),
                DateFormat::DdMmYyyy => "dd/mm/yyyy".to_string(),
                DateFormat::YyyyMmDdSlash => "yyyy/mm/dd".to_string(),
                DateFormat::MmmDdYyyy => "mmm dd, yyyy".to_string(),
                DateFormat::DdMmmYyyy => "dd-mmm-yyyy".to_string(),
            },
            NumberFormat::Time(tf) => match tf {
                TimeFormat::HhMmSs24 => "hh:mm:ss".to_string(),
                TimeFormat::HhMm24 => "hh:mm".to_string(),
                TimeFormat::HhMmSs12 => "hh:mm:ss AM/PM".to_string(),
                TimeFormat::HhMm12 => "hh:mm AM/PM".to_string(),
            },
            NumberFormat::DateTime(dtf) => match dtf {
                DateTimeFormat::Iso8601 => "yyyy-mm-dd hh:mm:ss".to_string(),
                DateTimeFormat::UsStandard => "mm/dd/yyyy hh:mm:ss".to_string(),
            },
            NumberFormat::Text => "@".to_string(),
            NumberFormat::Custom(s) => s.clone(),
        }
    }

    pub fn from_format_string(s: &str) -> Self {
        let trimmed = s.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("general") {
            return Self::General;
        }
        if trimmed == "@" {
            return Self::Text;
        }

        // Percentage check
        if trimmed.ends_with('%') {
            let core = trimmed.trim_end_matches('%');
            let decimals = if let Some((_, decimals)) = core.split_once('.') {
                decimals.chars().filter(|c| *c == '0' || *c == '#').count() as u8
            } else {
                0
            };
            return Self::Percentage { decimal_places: decimals };
        }

        // Date check
        let lower = trimmed.to_ascii_lowercase();
        if lower == "yyyy-mm-dd" {
            return Self::Date(DateFormat::YyyyMmDd);
        } else if lower == "mm/dd/yyyy" {
            return Self::Date(DateFormat::MmDdYyyy);
        } else if lower == "dd/mm/yyyy" {
            return Self::Date(DateFormat::DdMmYyyy);
        } else if lower == "yyyy/mm/dd" {
            return Self::Date(DateFormat::YyyyMmDdSlash);
        } else if lower == "hh:mm:ss" {
            return Self::Time(TimeFormat::HhMmSs24);
        } else if lower == "hh:mm" {
            return Self::Time(TimeFormat::HhMm24);
        } else if lower == "yyyy-mm-dd hh:mm:ss" {
            return Self::DateTime(DateTimeFormat::Iso8601);
        }

        // Currency check
        for sym in &["$", "€", "£", "¥", "₹"] {
            if trimmed.contains(sym) {
                let use_parens = trimmed.contains('(') && trimmed.contains(')');
                let neg = if use_parens {
                    NegativeNumberFormat::Parentheses
                } else {
                    NegativeNumberFormat::Minus
                };
                let decimals = if let Some(dot_idx) = trimmed.find('.') {
                    trimmed[dot_idx + 1..]
                        .chars()
                        .take_while(|c| *c == '0' || *c == '#')
                        .count() as u8
                } else {
                    0
                };
                return Self::Currency {
                    symbol: (*sym).to_string(),
                    decimal_places: decimals,
                    negative_format: neg,
                };
            }
        }

        // Number check
        if trimmed.contains('0') || trimmed.contains('#') {
            let thousands = trimmed.contains(',');
            let use_parens = trimmed.contains('(') && trimmed.contains(')');
            let neg = if use_parens {
                NegativeNumberFormat::Parentheses
            } else {
                NegativeNumberFormat::Minus
            };
            let decimals = if let Some(dot_idx) = trimmed.find('.') {
                trimmed[dot_idx + 1..]
                    .chars()
                    .take_while(|c| *c == '0' || *c == '#')
                    .count() as u8
            } else {
                0
            };
            return Self::Number {
                decimal_places: decimals,
                use_thousands_separator: thousands,
                negative_format: neg,
            };
        }

        Self::Custom(trimmed.to_string())
    }

    /// Format a numeric value according to this NumberFormat.
    pub fn format_number(&self, val: f64) -> String {
        match self {
            NumberFormat::General => {
                if val.fract() == 0.0 && val.abs() < 1e15 {
                    format!("{:.0}", val)
                } else {
                    let formatted = format!("{:.6}", val);
                    formatted.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            }
            NumberFormat::Number {
                decimal_places,
                use_thousands_separator,
                negative_format,
            } => {
                let is_neg = val < 0.0;
                let abs_val = val.abs();
                let num_str = format_with_decimals_and_sep(abs_val, *decimal_places, *use_thousands_separator);
                format_negative(&num_str, is_neg, *negative_format)
            }
            NumberFormat::Currency {
                symbol,
                decimal_places,
                negative_format,
            } => {
                let is_neg = val < 0.0;
                let abs_val = val.abs();
                let num_str = format_with_decimals_and_sep(abs_val, *decimal_places, true);
                let with_sym = format!("{symbol}{num_str}");
                format_negative(&with_sym, is_neg, *negative_format)
            }
            NumberFormat::Percentage { decimal_places } => {
                let pct = val * 100.0;
                format!("{:.prec$}%", pct, prec = *decimal_places as usize)
            }
            NumberFormat::Date(df) => {
                // If treated as Excel date serial number (days since 1899-12-30)
                if let Some(ndt) = excel_serial_to_datetime(val) {
                    format_date_only(&ndt, *df)
                } else {
                    val.to_string()
                }
            }
            NumberFormat::Time(tf) => {
                if let Some(ndt) = excel_serial_to_datetime(val) {
                    format_time_only(&ndt, *tf)
                } else {
                    val.to_string()
                }
            }
            NumberFormat::DateTime(dtf) => {
                if let Some(ndt) = excel_serial_to_datetime(val) {
                    format_datetime_full(&ndt, *dtf)
                } else {
                    val.to_string()
                }
            }
            NumberFormat::Text => val.to_string(),
            NumberFormat::Custom(_) => val.to_string(),
        }
    }

    /// Format a NaiveDateTime according to date/time format settings.
    pub fn format_datetime(&self, dt: &NaiveDateTime) -> String {
        match self {
            NumberFormat::Date(df) => format_date_only(dt, *df),
            NumberFormat::Time(tf) => format_time_only(dt, *tf),
            NumberFormat::DateTime(dtf) => format_datetime_full(dt, *dtf),
            _ => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

fn format_with_decimals_and_sep(val: f64, decimals: u8, use_sep: bool) -> String {
    let formatted = format!("{:.prec$}", val, prec = decimals as usize);
    if !use_sep {
        return formatted;
    }

    let (int_part, frac_part) = match formatted.split_once('.') {
        Some((int_p, frac_p)) => (int_p, Some(frac_p)),
        None => (formatted.as_str(), None),
    };

    let mut with_commas = String::new();
    let chars: Vec<char> = int_part.chars().collect();
    let len = chars.len();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            with_commas.push(',');
        }
        with_commas.push(ch);
    }

    if let Some(frac) = frac_part {
        format!("{}.{}", with_commas, frac)
    } else {
        with_commas
    }
}

fn format_negative(s: &str, is_neg: bool, neg_fmt: NegativeNumberFormat) -> String {
    if !is_neg {
        return s.to_string();
    }
    match neg_fmt {
        NegativeNumberFormat::Minus | NegativeNumberFormat::RedMinus => format!("-{}", s),
        NegativeNumberFormat::Parentheses | NegativeNumberFormat::RedParentheses => format!("({})", s),
    }
}

pub fn excel_serial_to_datetime(serial: f64) -> Option<NaiveDateTime> {
    if serial < 0.0 || serial > 2_958_465.0 {
        return None;
    }
    // Excel erroneously treats 1900 as a leap year. Day 60 is Feb 29, 1900.
    let days = if serial >= 61.0 {
        serial.floor() as i64 - 2
    } else if serial >= 1.0 {
        serial.floor() as i64 - 1
    } else {
        0
    };

    let base = NaiveDate::from_ymd_opt(1900, 1, 1)?;
    let date = base.checked_add_signed(chrono::Duration::days(days))?;

    let frac = serial.fract();
    let total_seconds = (frac * 86400.0).round() as u32;
    let hours = (total_seconds / 3600) % 24;
    let mins = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    let time = chrono::NaiveTime::from_hms_opt(hours, mins, secs)?;
    Some(NaiveDateTime::new(date, time))
}

pub fn datetime_to_excel_serial(dt: &NaiveDateTime) -> f64 {
    let base = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
    let days = dt.date().signed_duration_since(base).num_days();
    let adjusted_days = if days >= 59 { days + 2 } else { days + 1 };
    let time = dt.time();
    let frac = (time.hour() as f64 * 3600.0 + time.minute() as f64 * 60.0 + time.second() as f64) / 86400.0;
    adjusted_days as f64 + frac
}

fn format_date_only(dt: &NaiveDateTime, df: DateFormat) -> String {
    match df {
        DateFormat::YyyyMmDd => dt.format("%Y-%m-%d").to_string(),
        DateFormat::MmDdYyyy => dt.format("%m/%d/%Y").to_string(),
        DateFormat::DdMmYyyy => dt.format("%d/%m/%Y").to_string(),
        DateFormat::YyyyMmDdSlash => dt.format("%Y/%m/%d").to_string(),
        DateFormat::MmmDdYyyy => dt.format("%b %d, %Y").to_string(),
        DateFormat::DdMmmYyyy => dt.format("%d-%b-%Y").to_string(),
    }
}

fn format_time_only(dt: &NaiveDateTime, tf: TimeFormat) -> String {
    match tf {
        TimeFormat::HhMmSs24 => dt.format("%H:%M:%S").to_string(),
        TimeFormat::HhMm24 => dt.format("%H:%M").to_string(),
        TimeFormat::HhMmSs12 => dt.format("%I:%M:%S %p").to_string(),
        TimeFormat::HhMm12 => dt.format("%I:%M %p").to_string(),
    }
}

fn format_datetime_full(dt: &NaiveDateTime, dtf: DateTimeFormat) -> String {
    match dtf {
        DateTimeFormat::Iso8601 => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        DateTimeFormat::UsStandard => dt.format("%m/%d/%Y %H:%M:%S").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_formatting() {
        let fmt = NumberFormat::number(2, true);
        assert_eq!(fmt.format_number(1234567.89), "1,234,567.89");
        assert_eq!(fmt.format_number(-42.5), "-42.50");

        let cur = NumberFormat::currency("$", 2);
        assert_eq!(cur.format_number(1500.0), "$1,500.00");

        let pct = NumberFormat::percentage(1);
        assert_eq!(pct.format_number(0.125), "12.5%");

        let parens = NumberFormat::Number {
            decimal_places: 2,
            use_thousands_separator: true,
            negative_format: NegativeNumberFormat::Parentheses,
        };
        assert_eq!(parens.format_number(-100.0), "(100.00)");
    }

    #[test]
    fn test_format_parsing() {
        let parsed_pct = NumberFormat::from_format_string("0.00%");
        assert_eq!(parsed_pct, NumberFormat::Percentage { decimal_places: 2 });

        let parsed_num = NumberFormat::from_format_string("#,##0.00");
        assert_eq!(
            parsed_num,
            NumberFormat::Number {
                decimal_places: 2,
                use_thousands_separator: true,
                negative_format: NegativeNumberFormat::Minus,
            }
        );

        let parsed_cur = NumberFormat::from_format_string("$#,##0.00");
        assert_eq!(
            parsed_cur,
            NumberFormat::Currency {
                symbol: "$".to_string(),
                decimal_places: 2,
                negative_format: NegativeNumberFormat::Minus,
            }
        );
    }
}
