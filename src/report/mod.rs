//! Reporting utilities for summarising alpha evaluation results.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::alpha::AlphaEvaluation;

/// Summary row used when exporting evaluations to tabular formats.
#[derive(Debug, Clone, Serialize)]
pub struct AlphaSummaryRow {
    /// Feature identifier.
    pub feature_name: String,
    /// Alpha model identifier.
    pub model_name: String,
    /// Information Coefficient.
    pub ic: f64,
    /// Sharpe ratio of the sign-based signal.
    pub sharpe: f64,
    /// Mean sign-based return.
    pub mean_return: f64,
    /// Number of samples used during evaluation.
    pub sample_size: usize,
}

/// Report container capable of exporting evaluation results.
#[derive(Debug, Clone)]
pub struct AlphaReport {
    evaluations: Vec<AlphaEvaluation>,
}

impl AlphaReport {
    /// Create a report from raw evaluations.
    pub fn from_evaluations(evaluations: Vec<AlphaEvaluation>) -> Self {
        Self { evaluations }
    }

    /// Number of evaluations contained in the report.
    pub fn len(&self) -> usize {
        self.evaluations.len()
    }

    /// Whether the report is empty.
    pub fn is_empty(&self) -> bool {
        self.evaluations.is_empty()
    }

    /// Generate a list of summary rows for presentation or export.
    pub fn summary_rows(&self) -> Vec<AlphaSummaryRow> {
        self.evaluations
            .iter()
            .map(|evaluation| AlphaSummaryRow {
                feature_name: evaluation.feature_name.clone(),
                model_name: evaluation.model_name.clone(),
                ic: evaluation.ic,
                sharpe: evaluation.sharpe,
                mean_return: evaluation.mean_return,
                sample_size: evaluation.sample_size,
            })
            .collect()
    }

    /// Return top `limit` evaluations ranked by absolute IC.
    pub fn best_by_ic(&self, limit: usize) -> Vec<&AlphaEvaluation> {
        let mut refs: Vec<&AlphaEvaluation> = self.evaluations.iter().collect();
        refs.sort_by(|a, b| {
            b.ic.abs()
                .partial_cmp(&a.ic.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        refs.truncate(limit);
        refs
    }

    /// Write the report as a CSV file.
    pub fn write_csv<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writeln!(
            writer,
            "feature_name,model_name,ic,sharpe,mean_return,sample_size"
        )?;
        for row in self.summary_rows() {
            writeln!(
                writer,
                "{},{},{:.6},{:.6},{:.6},{}",
                row.feature_name,
                row.model_name,
                row.ic,
                row.sharpe,
                row.mean_return,
                row.sample_size
            )?;
        }
        writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alpha::AlphaEvaluation;

    #[test]
    fn report_generates_summary_rows() {
        let evaluation = AlphaEvaluation {
            feature_name: "feature".to_string(),
            model_name: "model".to_string(),
            ic: 0.2,
            sharpe: 1.1,
            mean_return: 0.01,
            scores: vec![0.1, -0.2],
            ic_series: vec![0.0, 0.0],
            sample_size: 2,
        };

        let report = AlphaReport::from_evaluations(vec![evaluation]);
        let summary = report.summary_rows();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].feature_name, "feature");
    }
}
