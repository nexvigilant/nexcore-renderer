//! Confidence propagation for GROUNDED reasoning.
//!
//! Implements `Uncertain<T>` — a wrapper that tracks confidence,
//! evidence count, and provenance for any value. This is Feature 1
//! from the GROUNDED specification: every claim carries its
//! epistemic status.
//!
//! ## Tier Classification
//!
//! - `Uncertain<T>`: T2-P (newtype wrapper adding confidence metadata)
//! - `ConfidenceSource`: T2-C (composed enum with provenance)

use std::fmt;

/// Tier: T2-P — Wraps any value with confidence metadata.
///
/// Every output from the GROUNDED loop carries its epistemic status:
/// how confident we are, how many observations support it, and where
/// the confidence estimate came from.
#[derive(Debug, Clone)]
pub struct Uncertain<T> {
    /// The wrapped value.
    pub value: T,
    /// Confidence level in \[0.0, 1.0\].
    pub confidence: f64,
    /// Number of observations supporting this value.
    pub evidence_count: u32,
    /// How the confidence was established.
    pub source: ConfidenceSource,
}

/// Tier: T2-C — Provenance of a confidence estimate.
#[derive(Debug, Clone)]
pub enum ConfidenceSource {
    /// Initial estimate before any observations.
    Prior,
    /// Derived from N direct observations.
    Observed(u32),
    /// Computed from other `Uncertain` values.
    Computed {
        /// Names of source uncertainties.
        from: Vec<String>,
    },
}

impl<T> Uncertain<T> {
    /// Create a new uncertain value with prior confidence.
    #[must_use]
    pub fn prior(value: T, confidence: f64) -> Self {
        Self {
            value,
            confidence: confidence.clamp(0.0, 1.0),
            evidence_count: 0,
            source: ConfidenceSource::Prior,
        }
    }

    /// Create from an observation.
    #[must_use]
    pub fn observed(value: T, confidence: f64, count: u32) -> Self {
        Self {
            value,
            confidence: confidence.clamp(0.0, 1.0),
            evidence_count: count,
            source: ConfidenceSource::Observed(count),
        }
    }

    /// Create from computation over other uncertain values.
    #[must_use]
    pub fn computed(value: T, confidence: f64, sources: Vec<String>) -> Self {
        Self {
            value,
            confidence: confidence.clamp(0.0, 1.0),
            evidence_count: 0,
            source: ConfidenceSource::Computed { from: sources },
        }
    }

    /// Map the inner value, preserving confidence metadata.
    #[must_use]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Uncertain<U> {
        Uncertain {
            value: f(self.value),
            confidence: self.confidence,
            evidence_count: self.evidence_count,
            source: self.source,
        }
    }

    /// Update confidence after a new observation.
    ///
    /// Uses Bayesian-style update: confidence moves toward `observation_support`
    /// weighted by the strength of the new evidence relative to existing.
    pub fn update(&mut self, observation_support: f64) {
        let support = observation_support.clamp(0.0, 1.0);
        self.evidence_count += 1;
        let n = f64::from(self.evidence_count);
        // Weighted moving average: existing evidence vs new observation
        self.confidence = (self.confidence * (n - 1.0) + support) / n;
        self.source = ConfidenceSource::Observed(self.evidence_count);
    }

    /// Whether this value has meaningful confidence (> 0.5).
    #[must_use]
    pub fn is_confident(&self) -> bool {
        self.confidence > 0.5
    }

    /// Whether this value has high confidence (> 0.8).
    #[must_use]
    pub fn is_highly_confident(&self) -> bool {
        self.confidence > 0.8
    }

    /// Confidence as a human-readable label.
    #[must_use]
    pub fn confidence_label(&self) -> &'static str {
        match self.confidence {
            c if c >= 0.9 => "very high",
            c if c >= 0.7 => "high",
            c if c >= 0.5 => "moderate",
            c if c >= 0.3 => "low",
            _ => "very low",
        }
    }
}

/// Combine two uncertain values, propagating confidence as the minimum
/// (weakest-link principle).
#[must_use]
pub fn combine_confidence<A, B>(a: &Uncertain<A>, b: &Uncertain<B>) -> f64 {
    a.confidence.min(b.confidence)
}

impl<T: fmt::Display> fmt::Display for Uncertain<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (confidence: {:.0}%, {} evidence)",
            self.value,
            self.confidence * 100.0,
            self.evidence_count
        )
    }
}

impl fmt::Display for ConfidenceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prior => write!(f, "prior"),
            Self::Observed(n) => write!(f, "observed ({n} samples)"),
            Self::Computed { from } => write!(f, "computed from: {}", from.join(", ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prior_creation() {
        let u = Uncertain::prior(42, 0.5);
        assert_eq!(u.value, 42);
        assert!((u.confidence - 0.5).abs() < f64::EPSILON);
        assert_eq!(u.evidence_count, 0);
        assert!(matches!(u.source, ConfidenceSource::Prior));
    }

    #[test]
    fn test_confidence_clamping() {
        let u = Uncertain::prior(1, 2.0);
        assert!((u.confidence - 1.0).abs() < f64::EPSILON);

        let u = Uncertain::prior(1, -1.0);
        assert!(u.confidence.abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_moves_confidence() {
        let mut u = Uncertain::prior(10, 0.5);
        u.update(1.0); // Strong positive evidence
        assert!(u.confidence > 0.5);
        assert_eq!(u.evidence_count, 1);
    }

    #[test]
    fn test_bayesian_convergence() {
        let mut u = Uncertain::prior(0, 0.5);
        for _ in 0..100 {
            u.update(0.9);
        }
        // After 100 observations of 0.9, confidence should be near 0.9
        assert!(u.confidence > 0.85);
    }

    #[test]
    fn test_map_preserves_metadata() {
        let u = Uncertain::observed(5, 0.8, 3);
        let mapped = u.map(|v| v * 2);
        assert_eq!(mapped.value, 10);
        assert!((mapped.confidence - 0.8).abs() < f64::EPSILON);
        assert_eq!(mapped.evidence_count, 3);
    }

    #[test]
    fn test_combine_confidence() {
        let a = Uncertain::prior(1, 0.9);
        let b = Uncertain::prior(2, 0.6);
        let combined = combine_confidence(&a, &b);
        assert!((combined - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_confidence_labels() {
        assert_eq!(Uncertain::prior(0, 0.95).confidence_label(), "very high");
        assert_eq!(Uncertain::prior(0, 0.75).confidence_label(), "high");
        assert_eq!(Uncertain::prior(0, 0.55).confidence_label(), "moderate");
        assert_eq!(Uncertain::prior(0, 0.35).confidence_label(), "low");
        assert_eq!(Uncertain::prior(0, 0.1).confidence_label(), "very low");
    }

    #[test]
    fn test_display_formatting() {
        let u = Uncertain::observed(42, 0.73, 5);
        let s = format!("{u}");
        assert!(s.contains("42"));
        assert!(s.contains("73%"));
        assert!(s.contains("5 evidence"));
    }
}
