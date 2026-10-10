//! Data validation engine for Excel cells and ranges.
//!
//! Provides rule definitions (List, Whole Number, Decimal, Date Range, Text Length),
//! validation evaluator against `CellValue`, and user-friendly error message generation.

use chrono::NaiveDate;
use exel_cell::CellValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("Validation failed: {message}")]
    Rejected {
        message: String,
        style: ValidationErrorStyle,
    },
}

/// Severity style of validation error dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationErrorStyle {
    Stop,
    Warning,
    Information,
}

/// Comparison operators for numerical, date, and length criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Between,
    NotBetween,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
}

impl ComparisonOperator {
    pub fn matches_f64(&self, val: f64, min: f64, max: Option<f64>) -> bool {
        match self {
            Self::Between => {
                let upper = max.unwrap_or(min);
                val >= min && val <= upper
            }
            Self::NotBetween => {
                let upper = max.unwrap_or(min);
                val < min || val > upper
            }
            Self::Equal => (val - min).abs() < f64::EPSILON,
            Self::NotEqual => (val - min).abs() >= f64::EPSILON,
            Self::GreaterThan => val > min,
            Self::LessThan => val < min,
            Self::GreaterOrEqual => val >= min,
            Self::LessOrEqual => val <= min,
        }
    }

    pub fn matches_date(&self, val: NaiveDate, start: NaiveDate, end: Option<NaiveDate>) -> bool {
        match self {
            Self::Between => {
                let upper = end.unwrap_or(start);
                val >= start && val <= upper
            }
            Self::NotBetween => {
                let upper = end.unwrap_or(start);
                val < start || val > upper
            }
            Self::Equal => val == start,
            Self::NotEqual => val != start,
            Self::GreaterThan => val > start,
            Self::LessThan => val < start,
            Self::GreaterOrEqual => val >= start,
            Self::LessOrEqual => val <= start,
        }
    }
}

/// Allowed data criteria for cell input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationCriteria {
    /// Restricts input to whole integers satisfying a comparison.
    WholeNumber {
        operator: ComparisonOperator,
        min: i64,
        max: Option<i64>,
    },
    /// Restricts input to decimals/floating point numbers satisfying a comparison.
    Decimal {
        operator: ComparisonOperator,
        min: f64,
        max: Option<f64>,
    },
    /// Restricts input to an explicit list of acceptable values.
    List {
        values: Vec<String>,
        ignore_case: bool,
    },
    /// Restricts input to a calendar date range.
    DateRange {
        operator: ComparisonOperator,
        start: NaiveDate,
        end: Option<NaiveDate>,
    },
    /// Restricts length of text string.
    TextLength {
        operator: ComparisonOperator,
        min: usize,
        max: Option<usize>,
    },
}

/// A comprehensive data validation rule attached to cells or ranges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataValidationRule {
    pub criteria: ValidationCriteria,
    pub allow_blank: bool,
    pub error_style: ValidationErrorStyle,
    pub error_title: Option<String>,
    pub error_message: Option<String>,
    pub prompt_title: Option<String>,
    pub prompt_message: Option<String>,
}

impl DataValidationRule {
    pub fn new(criteria: ValidationCriteria) -> Self {
        Self {
            criteria,
            allow_blank: true,
            error_style: ValidationErrorStyle::Stop,
            error_title: None,
            error_message: None,
            prompt_title: None,
            prompt_message: None,
        }
    }

    pub fn with_allow_blank(mut self, allow: bool) -> Self {
        self.allow_blank = allow;
        self
    }

    pub fn with_error_message<S1: Into<String>, S2: Into<String>>(
        mut self,
        title: S1,
        message: S2,
    ) -> Self {
        self.error_title = Some(title.into());
        self.error_message = Some(message.into());
        self
    }

    /// Evaluates `cell_value` against this validation rule.
    /// Returns `Ok(())` if valid, or `Err(ValidationError)` if invalid.
    pub fn validate(&self, cell_value: &CellValue) -> Result<(), ValidationError> {
        if self.allow_blank && matches!(cell_value, CellValue::Empty) {
            return Ok(());
        }

        let is_valid = match &self.criteria {
            ValidationCriteria::WholeNumber { operator, min, max } => {
                if let Some(n) = cell_value.as_number() {
                    if n.fract().abs() < f64::EPSILON {
                        let i = n as i64;
                        operator.matches_f64(i as f64, *min as f64, max.map(|m| m as f64))
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            ValidationCriteria::Decimal { operator, min, max } => {
                if let Some(n) = cell_value.as_number() {
                    operator.matches_f64(n, *min, *max)
                } else {
                    false
                }
            }

            ValidationCriteria::List {
                values,
                ignore_case,
            } => {
                let text = cell_value.to_display_string();
                if *ignore_case {
                    values.iter().any(|v| v.eq_ignore_ascii_case(text.trim()))
                } else {
                    values.iter().any(|v| v == text.trim())
                }
            }

            ValidationCriteria::DateRange {
                operator,
                start,
                end,
            } => match cell_value {
                CellValue::DateTime(dt) => operator.matches_date(dt.date(), *start, *end),
                CellValue::Text(s) => {
                    if let Ok(d) = NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d") {
                        operator.matches_date(d, *start, *end)
                    } else {
                        false
                    }
                }
                _ => false,
            },

            ValidationCriteria::TextLength { operator, min, max } => {
                let text = cell_value.to_display_string();
                let len = text.chars().count();
                operator.matches_f64(len as f64, *min as f64, max.map(|m| m as f64))
            }
        };

        if is_valid {
            Ok(())
        } else {
            let message = self
                .error_message
                .clone()
                .unwrap_or_else(|| self.generate_default_error_message(cell_value));

            Err(ValidationError::Rejected {
                message,
                style: self.error_style,
            })
        }
    }

    /// Generates a descriptive default error message when no custom message is set.
    pub fn generate_default_error_message(&self, val: &CellValue) -> String {
        match &self.criteria {
            ValidationCriteria::WholeNumber { operator, min, max } => match operator {
                ComparisonOperator::Between => format!(
                    "Value '{}' is invalid. Must be an integer between {} and {}.",
                    val.to_display_string(),
                    min,
                    max.unwrap_or(*min)
                ),
                ComparisonOperator::GreaterThan => {
                    format!("Value must be an integer greater than {min}.")
                }
                _ => "Value must satisfy whole number constraint.".to_string(),
            },
            ValidationCriteria::Decimal { operator, min, max } => match operator {
                ComparisonOperator::Between => format!(
                    "Value must be a decimal between {} and {}.",
                    min,
                    max.unwrap_or(*min)
                ),
                _ => "Value must satisfy decimal constraint.".to_string(),
            },
            ValidationCriteria::List { values, .. } => {
                format!(
                    "Value '{}' is not in allowed list: {}.",
                    val.to_display_string(),
                    values.join(", ")
                )
            }
            ValidationCriteria::DateRange {
                operator,
                start,
                end,
            } => match operator {
                ComparisonOperator::Between => format!(
                    "Date must be between {} and {}.",
                    start,
                    end.unwrap_or(*start)
                ),
                _ => "Date must satisfy range constraint.".to_string(),
            },
            ValidationCriteria::TextLength { min, max, .. } => format!(
                "Text length must be between {} and {} characters.",
                min,
                max.unwrap_or(*min)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whole_number_validation() {
        let rule = DataValidationRule::new(ValidationCriteria::WholeNumber {
            operator: ComparisonOperator::Between,
            min: 1,
            max: Some(100),
        });

        assert!(rule.validate(&CellValue::Number(50.0)).is_ok());
        assert!(rule.validate(&CellValue::Number(1.0)).is_ok());
        assert!(rule.validate(&CellValue::Number(100.0)).is_ok());

        // Decimals rejected for whole number
        assert!(rule.validate(&CellValue::Number(50.5)).is_err());
        // Out of range
        assert!(rule.validate(&CellValue::Number(101.0)).is_err());
        assert!(rule.validate(&CellValue::Number(0.0)).is_err());
        // Strings rejected
        assert!(rule.validate(&CellValue::Text("Fifty".into())).is_err());
    }

    #[test]
    fn test_decimal_validation() {
        let rule = DataValidationRule::new(ValidationCriteria::Decimal {
            operator: ComparisonOperator::GreaterThan,
            min: 0.0,
            max: None,
        });

        assert!(rule.validate(&CellValue::Number(0.001)).is_ok());
        assert!(rule.validate(&CellValue::Number(42.5)).is_ok());
        assert!(rule.validate(&CellValue::Number(0.0)).is_err());
        assert!(rule.validate(&CellValue::Number(-5.0)).is_err());
    }

    #[test]
    fn test_list_validation() {
        let rule = DataValidationRule::new(ValidationCriteria::List {
            values: vec!["Pending".into(), "Approved".into(), "Rejected".into()],
            ignore_case: true,
        });

        assert!(rule.validate(&CellValue::Text("Approved".into())).is_ok());
        assert!(rule.validate(&CellValue::Text("approved".into())).is_ok());
        assert!(rule.validate(&CellValue::Text("PENDING".into())).is_ok());

        let err = rule.validate(&CellValue::Text("Cancelled".into()));
        assert!(err.is_err());
        if let Err(ValidationError::Rejected { message, .. }) = err {
            assert!(message.contains("Pending, Approved, Rejected"));
        }
    }

    #[test]
    fn test_date_range_validation() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();

        let rule = DataValidationRule::new(ValidationCriteria::DateRange {
            operator: ComparisonOperator::Between,
            start,
            end: Some(end),
        });

        let valid_dt = CellValue::DateTime(start.and_hms_opt(12, 0, 0).unwrap());
        assert!(rule.validate(&valid_dt).is_ok());

        let valid_str = CellValue::Text("2025-06-15".into());
        assert!(rule.validate(&valid_str).is_ok());

        let invalid_str = CellValue::Text("2026-01-01".into());
        assert!(rule.validate(&invalid_str).is_err());
    }

    #[test]
    fn test_allow_blank() {
        let rule = DataValidationRule::new(ValidationCriteria::WholeNumber {
            operator: ComparisonOperator::Equal,
            min: 10,
            max: None,
        })
        .with_allow_blank(true);

        assert!(rule.validate(&CellValue::Empty).is_ok());

        let strict_rule = rule.with_allow_blank(false);
        assert!(strict_rule.validate(&CellValue::Empty).is_err());
    }
}
