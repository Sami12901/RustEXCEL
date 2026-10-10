//! Data analysis and statistics tools for Rust EXCEL.
//!
//! Provides descriptive statistics (mean, median, sample std dev, min, max, count, range),
//! frequency tables (histogram bins), and categorical frequency distributions for worksheet ranges.

use exel_cell::{CellCoord, CellRange, CellValue};
use exel_workbook::Worksheet;
use serde::{Deserialize, Serialize};

/// Summary of descriptive statistical measures for a numeric sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescriptiveStatistics {
    pub count: usize,
    pub sum: f64,
    pub mean: f64,
    pub median: f64,
    pub sample_std_dev: Option<f64>,
    pub sample_variance: Option<f64>,
    pub min: f64,
    pub max: f64,
    pub range: f64,
}

/// A single bucket in a numerical frequency table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrequencyBin {
    pub bin_label: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub frequency: usize,
    pub relative_frequency: f64,
    pub cumulative_frequency: usize,
}

/// A complete frequency distribution table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrequencyTable {
    pub bins: Vec<FrequencyBin>,
    pub total_count: usize,
}

/// Frequency count for discrete categorical values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoricalFrequency {
    pub value: String,
    pub count: usize,
    pub percentage: f64,
}

/// Computes descriptive statistics for a numeric slice.
pub fn compute_descriptive_statistics(values: &[f64]) -> Option<DescriptiveStatistics> {
    let mut finite_vals: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite_vals.is_empty() {
        return None;
    }

    finite_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let count = finite_vals.len();
    let sum: f64 = finite_vals.iter().sum();
    let mean = sum / (count as f64);

    let median = if count % 2 == 1 {
        finite_vals[count / 2]
    } else {
        (finite_vals[(count / 2) - 1] + finite_vals[count / 2]) / 2.0
    };

    let min = *finite_vals.first().unwrap();
    let max = *finite_vals.last().unwrap();
    let range = max - min;

    let (sample_variance, sample_std_dev) = if count > 1 {
        let var_sum: f64 = finite_vals.iter().map(|&x| (x - mean).powi(2)).sum();
        let var = var_sum / ((count - 1) as f64);
        (Some(var), Some(var.sqrt()))
    } else {
        (None, None)
    };

    Some(DescriptiveStatistics {
        count,
        sum,
        mean,
        median,
        sample_std_dev,
        sample_variance,
        min,
        max,
        range,
    })
}

/// Computes descriptive statistics for all numeric cells within a worksheet range.
pub fn compute_range_statistics(
    sheet: &Worksheet,
    range: CellRange,
) -> Option<DescriptiveStatistics> {
    let mut numbers = Vec::new();
    for r in range.start.row..=range.end.row {
        for c in range.start.col..=range.end.col {
            if let Ok(coord) = CellCoord::new(r, c) {
                if let Some(num) = sheet.get_cell_value(coord).as_number() {
                    numbers.push(num);
                }
            }
        }
    }
    compute_descriptive_statistics(&numbers)
}

/// Generates a numerical frequency distribution table divided into `bin_count` equal intervals.
pub fn generate_frequency_table(values: &[f64], bin_count: usize) -> Option<FrequencyTable> {
    let mut finite_vals: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite_vals.is_empty() || bin_count == 0 {
        return None;
    }

    finite_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = *finite_vals.first().unwrap();
    let max = *finite_vals.last().unwrap();
    let total_count = finite_vals.len();

    let step = if (max - min).abs() < f64::EPSILON {
        1.0
    } else {
        (max - min) / (bin_count as f64)
    };

    let mut bins = Vec::with_capacity(bin_count);
    let mut cum_freq = 0;

    for i in 0..bin_count {
        let lower = min + (i as f64) * step;
        let upper = if i == bin_count - 1 {
            max
        } else {
            min + ((i + 1) as f64) * step
        };

        let freq = finite_vals
            .iter()
            .filter(|&&x| {
                if i == bin_count - 1 {
                    x >= lower && x <= upper
                } else {
                    x >= lower && x < upper
                }
            })
            .count();

        cum_freq += freq;
        let rel_freq = (freq as f64) / (total_count as f64);

        bins.push(FrequencyBin {
            bin_label: format!("{:.2} - {:.2}", lower, upper),
            lower_bound: lower,
            upper_bound: upper,
            frequency: freq,
            relative_frequency: rel_freq,
            cumulative_frequency: cum_freq,
        });
    }

    Some(FrequencyTable { bins, total_count })
}

/// Generates a frequency table from numeric values within a worksheet range.
pub fn generate_frequency_table_from_range(
    sheet: &Worksheet,
    range: CellRange,
    bin_count: usize,
) -> Option<FrequencyTable> {
    let mut numbers = Vec::new();
    for r in range.start.row..=range.end.row {
        for c in range.start.col..=range.end.col {
            if let Ok(coord) = CellCoord::new(r, c) {
                if let Some(num) = sheet.get_cell_value(coord).as_number() {
                    numbers.push(num);
                }
            }
        }
    }
    generate_frequency_table(&numbers, bin_count)
}

/// Generates a categorical frequency distribution from any cells within a worksheet range.
pub fn generate_categorical_frequency(
    sheet: &Worksheet,
    range: CellRange,
) -> Vec<CategoricalFrequency> {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0;

    for r in range.start.row..=range.end.row {
        for c in range.start.col..=range.end.col {
            if let Ok(coord) = CellCoord::new(r, c) {
                let val = sheet.get_cell_value(coord);
                if !matches!(val, CellValue::Empty) {
                    let s = val.to_display_string();
                    *counts.entry(s).or_default() += 1;
                    total += 1;
                }
            }
        }
    }

    if total == 0 {
        return Vec::new();
    }

    counts
        .into_iter()
        .map(|(value, count)| CategoricalFrequency {
            percentage: (count as f64) / (total as f64) * 100.0,
            value,
            count,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use exel_cell::SheetId;

    #[test]
    fn test_descriptive_statistics_calc() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let stats = compute_descriptive_statistics(&data).unwrap();

        assert_eq!(stats.count, 5);
        assert_eq!(stats.sum, 150.0);
        assert_eq!(stats.mean, 30.0);
        assert_eq!(stats.median, 30.0);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 50.0);
        assert_eq!(stats.range, 40.0);
        assert_eq!(stats.sample_variance, Some(250.0));
        assert!((stats.sample_std_dev.unwrap() - 15.811388).abs() < 1e-4);
    }

    #[test]
    fn test_range_statistics() {
        let mut sheet = Worksheet::new(SheetId::new("s_data"), "Stats");
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Number(2.0));
        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Number(4.0));
        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Number(4.0));
        sheet.set_cell_value(CellCoord::must_new(4, 1), CellValue::Number(6.0));

        let range = CellRange::new(CellCoord::must_new(1, 1), CellCoord::must_new(4, 1));
        let stats = compute_range_statistics(&sheet, range).unwrap();

        assert_eq!(stats.count, 4);
        assert_eq!(stats.mean, 4.0);
        assert_eq!(stats.median, 4.0);
        assert_eq!(stats.min, 2.0);
        assert_eq!(stats.max, 6.0);
    }

    #[test]
    fn test_frequency_table_generation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let table = generate_frequency_table(&values, 2).unwrap();

        assert_eq!(table.total_count, 10);
        assert_eq!(table.bins.len(), 2);
        assert_eq!(table.bins[0].frequency, 5); // 1.0 to < 5.5
        assert_eq!(table.bins[1].frequency, 5); // 5.5 to 10.0
        assert_eq!(table.bins[1].cumulative_frequency, 10);
    }

    #[test]
    fn test_categorical_frequency() {
        let mut sheet = Worksheet::new(SheetId::new("s_cat"), "Cats");
        sheet.set_cell_value(CellCoord::must_new(1, 1), CellValue::Text("Yes".into()));
        sheet.set_cell_value(CellCoord::must_new(2, 1), CellValue::Text("No".into()));
        sheet.set_cell_value(CellCoord::must_new(3, 1), CellValue::Text("Yes".into()));
        sheet.set_cell_value(CellCoord::must_new(4, 1), CellValue::Text("Yes".into()));

        let range = CellRange::new(CellCoord::must_new(1, 1), CellCoord::must_new(4, 1));
        let freq = generate_categorical_frequency(&sheet, range);

        assert_eq!(freq.len(), 2);
        let yes = freq.iter().find(|f| f.value == "Yes").unwrap();
        assert_eq!(yes.count, 3);
        assert_eq!(yes.percentage, 75.0);

        let no = freq.iter().find(|f| f.value == "No").unwrap();
        assert_eq!(no.count, 1);
        assert_eq!(no.percentage, 25.0);
    }
}
