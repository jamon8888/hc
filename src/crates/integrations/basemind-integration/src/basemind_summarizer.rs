//! Basemind Summarizer Tool
//!
//! Provides summarization capabilities for basemind-documented content,
//! generating concise overviews while preserving key insights and citations.

use super::models::BasemindAnalysis;
use serde::{Deserialize, Serialize};

/// Summary of a basemind analysis session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BasemindSummary {
    /// Overall topic assessment
    pub overall_assessment: String,
    /// Key findings extracted
    pub key_findings: Vec<String>,
    /// Recommended actions
    pub recommendations: Vec<String>,
    /// Preserved citations and references
    pub citations: Vec<String>,
    /// Summary confidence score (0.0 - 1.0)
    pub confidence: f64,
}

impl BasemindSummary {
    /// Create a new summary from analysis data
    pub fn from_analysis(analysis: &BasemindAnalysis) -> Self {
        Self {
            overall_assessment: analysis.overall_assessment.clone(),
            key_findings: analysis.key_points.clone(),
            recommendations: vec![
                "Review key insights before action".to_string(),
                "Verify findings with primary sources".to_string(),
            ],
            citations: analysis.references.clone(),
            confidence: 0.85,
        }
    }
}

/// Generate a brief summary suitable for model prompts
pub fn generate_summary_prompt(analysis: &BasemindAnalysis) -> String {
    format!(
        "Summarize the following analysis in 2-3 sentences, in English:\n\
         Assessment: {}\n\
         Key points: {:?}\n\
         Action items: {:?}",
        analysis.overall_assessment,
        analysis.key_points,
        analysis.recommended_actions
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_creation() {
        let analysis = BasemindAnalysis {
            overall_assessment: "Test assessment".to_string(),
            key_points: vec!["Point 1".to_string(), "Point 2".to_string()],
            references: vec!["ref1".to_string()],
            recommended_actions: vec!["Action 1".to_string()],
        };

        let summary = BasemindSummary::from_analysis(&analysis);
        assert_eq!(summary.overall_assessment, "Test assessment");
        assert_eq!(summary.key_findings.len(), 2);
        assert_eq!(summary.confidence, 0.85);
    }

    #[test]
    fn test_summary_prompt_generation() {
        let analysis = BasemindAnalysis {
            overall_assessment: "Test assessment".to_string(),
            key_points: vec!["Point 1".to_string()],
            references: vec![],
            recommended_actions: vec![],
        };

        let prompt = generate_summary_prompt(&analysis);
        assert!(prompt.contains("Test assessment"));
        assert!(prompt.contains("Point 1"));
    }
}
