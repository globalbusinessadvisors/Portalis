//! Compatibility Analyzer
//!
//! Analyzes detected features and calculates translatability scores.

use super::feature_detector::{DetectedFeature, FeatureSet, FeatureSupport, FeatureCategory, FeatureSummary};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Compatibility analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub score: TranslatabilityScore,
    pub blockers: Vec<Blocker>,
    pub warnings: Vec<Warning>,
    pub recommendations: Vec<Recommendation>,
    pub file_analysis: HashMap<String, FileCompatibility>,
}

/// Translatability score (0-100%)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatabilityScore {
    pub overall: f64,
    pub by_category: HashMap<FeatureCategory, f64>,
    pub confidence: ConfidenceLevel,
}

/// Confidence in the score
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    High,    // > 90% of features analyzed
    Medium,  // 70-90% of features analyzed
    Low,     // < 70% of features analyzed
}

/// Translation blocker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    pub feature: String,
    pub category: FeatureCategory,
    pub count: usize,
    pub impact: BlockerImpact,
    pub description: String,
    pub workaround: Option<String>,
    pub locations: Vec<String>,
}

/// Impact level of a blocker
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockerImpact {
    Critical,  // Prevents translation entirely
    High,      // Prevents translation of specific modules
    Medium,    // Requires significant refactoring
    Low,       // Minor workaround needed
}

/// Warning about partial support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub feature: String,
    pub category: FeatureCategory,
    pub count: usize,
    pub description: String,
    pub limitation: String,
}

/// Recommendation for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationPriority {
    High,
    Medium,
    Low,
}

/// Per-file compatibility analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCompatibility {
    pub file_path: String,
    pub translatability: f64,
    pub features_total: usize,
    pub features_supported: usize,
    pub blockers: usize,
    pub warnings: usize,
}

/// Compatibility analyzer
pub struct CompatibilityAnalyzer {
    /// Minimum score to consider translatable
    _min_translatable_score: f64,
}

impl CompatibilityAnalyzer {
    pub fn new() -> Self {
        Self {
            _min_translatable_score: 70.0,
        }
    }

    /// Analyze feature set for compatibility
    pub fn analyze(&self, feature_set: &FeatureSet) -> CompatibilityReport {
        let score = self.calculate_score(feature_set);
        let blockers = self.identify_blockers(feature_set);
        let warnings = self.identify_warnings(feature_set);
        let recommendations = self.generate_recommendations(feature_set, &score, &blockers);

        CompatibilityReport {
            score,
            blockers,
            warnings,
            recommendations,
            file_analysis: HashMap::new(), // Will be populated per-file
        }
    }

    /// Analyze multiple files
    pub fn analyze_files(&self, file_features: &HashMap<String, FeatureSet>) -> CompatibilityReport {
        let mut all_features = Vec::new();
        let mut file_analysis = HashMap::new();

        // Collect all features and analyze per file
        for (file_path, features) in file_features {
            all_features.extend(features.features.clone());

            let file_score = self.calculate_file_score(features);
            let file_blockers = features.features.iter()
                .filter(|f| f.support == FeatureSupport::None)
                .count();
            let file_warnings = features.features.iter()
                .filter(|f| f.support == FeatureSupport::Partial)
                .count();

            file_analysis.insert(file_path.clone(), FileCompatibility {
                file_path: file_path.clone(),
                translatability: file_score,
                features_total: features.features.len(),
                features_supported: features.summary.fully_supported,
                blockers: file_blockers,
                warnings: file_warnings,
            });
        }

        // Create combined feature set
        let combined = FeatureSet {
            features: all_features.clone(),
            summary: self.summarize_features(&all_features),
        };

        let score = self.calculate_score(&combined);
        let blockers = self.identify_blockers(&combined);
        let warnings = self.identify_warnings(&combined);
        let recommendations = self.generate_recommendations(&combined, &score, &blockers);

        CompatibilityReport {
            score,
            blockers,
            warnings,
            recommendations,
            file_analysis,
        }
    }

    /// Calculate translatability score
    fn calculate_score(&self, features: &FeatureSet) -> TranslatabilityScore {
        let total = features.summary.total_features as f64;

        if total == 0.0 {
            return TranslatabilityScore {
                overall: 100.0,
                by_category: HashMap::new(),
                confidence: ConfidenceLevel::Low,
            };
        }

        // Weighted scoring: full support = 1.0, partial = 0.5, none = 0.0
        let score = (features.summary.fully_supported as f64
                    + features.summary.partially_supported as f64 * 0.5) / total * 100.0;

        // Calculate per-category scores
        let mut by_category = HashMap::new();
        for (category, _count) in &features.summary.by_category {
            let category_features: Vec<_> = features.features.iter()
                .filter(|f| &f.category == category)
                .collect();

            let category_total = category_features.len() as f64;
            let category_supported = category_features.iter()
                .filter(|f| f.support == FeatureSupport::Full)
                .count() as f64;
            let category_partial = category_features.iter()
                .filter(|f| f.support == FeatureSupport::Partial)
                .count() as f64;

            let category_score = if category_total > 0.0 {
                (category_supported + category_partial * 0.5) / category_total * 100.0
            } else {
                100.0
            };

            by_category.insert(category.clone(), category_score);
        }

        // Determine confidence based on feature coverage
        let confidence = if total > 100.0 {
            ConfidenceLevel::High
        } else if total > 20.0 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        };

        TranslatabilityScore {
            overall: score,
            by_category,
            confidence,
        }
    }

    /// Calculate score for a single file
    fn calculate_file_score(&self, features: &FeatureSet) -> f64 {
        let total = features.summary.total_features as f64;
        if total == 0.0 {
            return 100.0;
        }

        (features.summary.fully_supported as f64
         + features.summary.partially_supported as f64 * 0.5) / total * 100.0
    }

    /// Identify translation blockers
    fn identify_blockers(&self, features: &FeatureSet) -> Vec<Blocker> {
        let mut blockers = Vec::new();
        let mut blocker_map: HashMap<String, (usize, Vec<String>)> = HashMap::new();

        for feature in &features.features {
            if feature.support == FeatureSupport::None {
                let entry = blocker_map.entry(feature.name.clone()).or_insert((0, Vec::new()));
                entry.0 += feature.count;
                for loc in &feature.locations {
                    entry.1.push(format!("{}:{}", loc.file, loc.context));
                }
            }
        }

        for (name, (count, locations)) in blocker_map {
            let (impact, description, workaround) = self.classify_blocker(&name);

            blockers.push(Blocker {
                feature: name.clone(),
                category: self.get_category_for_blocker(&name),
                count,
                impact,
                description,
                workaround,
                locations,
            });
        }

        // Sort by impact (Critical first)
        blockers.sort_by(|a, b| {
            let order_a = match a.impact {
                BlockerImpact::Critical => 0,
                BlockerImpact::High => 1,
                BlockerImpact::Medium => 2,
                BlockerImpact::Low => 3,
            };
            let order_b = match b.impact {
                BlockerImpact::Critical => 0,
                BlockerImpact::High => 1,
                BlockerImpact::Medium => 2,
                BlockerImpact::Low => 3,
            };
            order_a.cmp(&order_b).then(b.count.cmp(&a.count))
        });

        blockers
    }

    /// Classify blocker impact
    fn classify_blocker(&self, feature_name: &str) -> (BlockerImpact, String, Option<String>) {
        if feature_name.contains("metaclass") {
            (
                BlockerImpact::Critical,
                "Metaclasses are not supported in Portalis. They require deep runtime introspection.".to_string(),
                Some("Refactor to use composition or regular classes with factory functions.".to_string()),
            )
        } else if feature_name == "eval" || feature_name == "exec" {
            (
                BlockerImpact::Critical,
                "Dynamic code execution is not supported in WASM environment.".to_string(),
                Some("Replace with static code or pre-compile all needed functionality.".to_string()),
            )
        } else if feature_name.contains("__getattr__") || feature_name.contains("__setattr__") {
            (
                BlockerImpact::High,
                "Dynamic attribute access is not fully supported.".to_string(),
                Some("Use explicit attributes or dictionary-based storage.".to_string()),
            )
        } else if feature_name == "abstractmethod" {
            (
                BlockerImpact::Medium,
                "Abstract methods require interface-like patterns.".to_string(),
                Some("Use trait-based design in Rust translation.".to_string()),
            )
        } else {
            (
                BlockerImpact::Low,
                format!("{} is not currently supported.", feature_name),
                None,
            )
        }
    }

    /// Get category for a blocker
    fn get_category_for_blocker(&self, name: &str) -> FeatureCategory {
        if name.contains("metaclass") {
            FeatureCategory::Metaclass
        } else if name == "eval" || name == "exec" {
            FeatureCategory::DynamicFeature
        } else if name.starts_with("__") && name.ends_with("__") {
            FeatureCategory::MagicMethod
        } else {
            FeatureCategory::Other
        }
    }

    /// Identify warnings for partial support
    fn identify_warnings(&self, features: &FeatureSet) -> Vec<Warning> {
        let mut warnings = Vec::new();
        let mut warning_map: HashMap<String, usize> = HashMap::new();

        for feature in &features.features {
            if feature.support == FeatureSupport::Partial {
                *warning_map.entry(feature.name.clone()).or_insert(0) += feature.count;
            }
        }

        for (name, count) in warning_map {
            let (description, limitation) = self.describe_partial_support(&name);

            warnings.push(Warning {
                feature: name.clone(),
                category: self.get_category_for_warning(&name),
                count,
                description,
                limitation,
            });
        }

        warnings
    }

    /// Describe partial support limitations
    fn describe_partial_support(&self, feature_name: &str) -> (String, String) {
        if feature_name.contains("async") {
            (
                "Async/await functionality is partially supported.".to_string(),
                "Limited to basic async functions. Complex async patterns may not work.".to_string(),
            )
        } else if feature_name == "dataclass" {
            (
                "Dataclasses have partial support.".to_string(),
                "Basic fields work, but advanced features (frozen, slots) may not.".to_string(),
            )
        } else if feature_name == "lru_cache" {
            (
                "LRU cache decorator has partial support.".to_string(),
                "Caching works but size limits may not be enforced.".to_string(),
            )
        } else {
            (
                format!("{} has partial support.", feature_name),
                "Some features may not work as expected.".to_string(),
            )
        }
    }

    /// Get category for a warning
    fn get_category_for_warning(&self, name: &str) -> FeatureCategory {
        if name.contains("async") {
            FeatureCategory::AsyncAwait
        } else if name == "dataclass" {
            FeatureCategory::Decorator
        } else {
            FeatureCategory::Other
        }
    }

    /// Generate recommendations
    fn generate_recommendations(
        &self,
        _features: &FeatureSet,
        score: &TranslatabilityScore,
        blockers: &[Blocker],
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Overall strategy recommendation
        if score.overall >= 90.0 {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::High,
                title: "Full Migration Recommended".to_string(),
                description: "Your codebase is highly compatible with Portalis.".to_string(),
                action_items: vec![
                    "Translate all modules at once for maximum benefit.".to_string(),
                    "Focus on testing to ensure behavioral equivalence.".to_string(),
                    "Consider parallel development during transition.".to_string(),
                ],
            });
        } else if score.overall >= 70.0 {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::High,
                title: "Incremental Migration Recommended".to_string(),
                description: "Your codebase is mostly compatible. Migrate in phases.".to_string(),
                action_items: vec![
                    "Start with highly compatible modules (90%+ score).".to_string(),
                    "Address blockers in critical modules first.".to_string(),
                    "Maintain Python fallbacks during transition.".to_string(),
                ],
            });
        } else if score.overall >= 50.0 {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::High,
                title: "Refactoring Required Before Migration".to_string(),
                description: "Significant blockers present. Refactor first.".to_string(),
                action_items: vec![
                    format!("Address {} critical blockers before migration.",
                            blockers.iter().filter(|b| b.impact == BlockerImpact::Critical).count()),
                    "Consider refactoring to eliminate unsupported patterns.".to_string(),
                    "Start with a small proof-of-concept module.".to_string(),
                ],
            });
        } else {
            recommendations.push(Recommendation {
                priority: RecommendationPriority::High,
                title: "Migration Not Recommended at This Time".to_string(),
                description: "Too many incompatibilities for successful migration.".to_string(),
                action_items: vec![
                    "Review blockers and consider if Portalis is the right solution.".to_string(),
                    "Alternatively, refactor heavily to remove unsupported features.".to_string(),
                    "Consider waiting for future Portalis versions with broader support.".to_string(),
                ],
            });
        }

        // Blocker-specific recommendations
        if !blockers.is_empty() {
            let critical_count = blockers.iter().filter(|b| b.impact == BlockerImpact::Critical).count();

            if critical_count > 0 {
                recommendations.push(Recommendation {
                    priority: RecommendationPriority::High,
                    title: format!("Address {} Critical Blockers", critical_count),
                    description: "These features prevent translation and must be resolved.".to_string(),
                    action_items: blockers.iter()
                        .filter(|b| b.impact == BlockerImpact::Critical)
                        .take(5)
                        .map(|b| format!("{}: {}", b.feature, b.description))
                        .collect(),
                });
            }
        }

        // Testing recommendations
        recommendations.push(Recommendation {
            priority: RecommendationPriority::Medium,
            title: "Comprehensive Testing Required".to_string(),
            description: "Ensure behavioral equivalence through testing.".to_string(),
            action_items: vec![
                "Create test suite covering all translated functionality.".to_string(),
                "Use property-based testing for complex behaviors.".to_string(),
                "Validate WASM output against Python reference implementation.".to_string(),
            ],
        });

        recommendations
    }

    /// Summarize features manually
    fn summarize_features(&self, features: &[DetectedFeature]) -> FeatureSummary {

        let total_features = features.len();
        let fully_supported = features.iter().filter(|f| f.support == FeatureSupport::Full).count();
        let partially_supported = features.iter().filter(|f| f.support == FeatureSupport::Partial).count();
        let unsupported = features.iter().filter(|f| f.support == FeatureSupport::None).count();

        let mut by_category = HashMap::new();
        for feature in features {
            *by_category.entry(feature.category.clone()).or_insert(0) += 1;
        }

        FeatureSummary {
            total_features,
            fully_supported,
            partially_supported,
            unsupported,
            by_category,
        }
    }
}

impl Default for CompatibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assessment::feature_detector::FeatureLocation;

    #[test]
    fn test_score_all_supported() {
        let analyzer = CompatibilityAnalyzer::new();
        let features = FeatureSet {
            features: vec![],
            summary: FeatureSummary {
                total_features: 100,
                fully_supported: 100,
                partially_supported: 0,
                unsupported: 0,
                by_category: HashMap::new(),
            },
        };

        let report = analyzer.analyze(&features);
        assert_eq!(report.score.overall, 100.0);
    }

    #[test]
    fn test_score_partial_support() {
        let analyzer = CompatibilityAnalyzer::new();
        let features = FeatureSet {
            features: vec![],
            summary: FeatureSummary {
                total_features: 100,
                fully_supported: 50,
                partially_supported: 50,
                unsupported: 0,
                by_category: HashMap::new(),
            },
        };

        let report = analyzer.analyze(&features);
        assert_eq!(report.score.overall, 75.0); // 50 + 25 (50% of 50)
    }

    #[test]
    fn test_score_with_blockers() {
        let analyzer = CompatibilityAnalyzer::new();
        let features = FeatureSet {
            features: vec![],
            summary: FeatureSummary {
                total_features: 100,
                fully_supported: 50,
                partially_supported: 25,
                unsupported: 25,
                by_category: HashMap::new(),
            },
        };

        let report = analyzer.analyze(&features);
        assert_eq!(report.score.overall, 62.5); // (50 + 12.5) / 100 * 100
    }

    #[test]
    fn test_identify_blockers() {
        let analyzer = CompatibilityAnalyzer::new();
        let features = FeatureSet {
            features: vec![
                DetectedFeature {
                    category: FeatureCategory::Metaclass,
                    name: "metaclass".to_string(),
                    support: FeatureSupport::None,
                    count: 1,
                    locations: vec![FeatureLocation {
                        file: "test.py".to_string(),
                        line: Some(10),
                        context: "class Meta(type)".to_string(),
                    }],
                    details: None,
                },
            ],
            summary: FeatureSummary {
                total_features: 1,
                fully_supported: 0,
                partially_supported: 0,
                unsupported: 1,
                by_category: HashMap::new(),
            },
        };

        let report = analyzer.analyze(&features);
        assert_eq!(report.blockers.len(), 1);
        assert_eq!(report.blockers[0].impact, BlockerImpact::Critical);
    }
}
