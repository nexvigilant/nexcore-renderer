//! Hypothesis, Experiment, and Outcome types for the GROUNDED loop.
//!
//! These types implement the scientific method cycle:
//! Hypothesis → Experiment → Outcome → Learning
//!
//! ## Tier Classification
//!
//! - `HypothesisId`, `ExperienceId`: T2-P (newtypes over u64)
//! - `Hypothesis`, `Experiment`, `Outcome`, `Learning`: T3 (domain-specific)
//! - `HypothesisStatus`, `ExperimentMethod`: T2-C (composed enums)

use super::confidence::Uncertain;
use std::fmt;

/// Tier: T2-P — Unique identifier for a hypothesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HypothesisId(pub u64);

/// Tier: T2-P — Unique identifier for an experience record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExperienceId(pub u64);

/// Tier: T3 — A testable claim about the world.
///
/// Every hypothesis must specify how it could be falsified
/// (Popperian criterion). Confidence is tracked via `Uncertain<f64>`.
#[derive(Debug, Clone)]
pub struct Hypothesis {
    /// Unique identifier.
    pub id: HypothesisId,
    /// The claim being tested.
    pub claim: String,
    /// Current confidence in the claim.
    pub confidence: Uncertain<f64>,
    /// How this hypothesis could be disproved.
    pub falsification: String,
    /// Context that led to this hypothesis.
    pub context: Vec<String>,
    /// Current lifecycle status.
    pub status: HypothesisStatus,
}

/// Tier: T2-C — Lifecycle status of a hypothesis.
#[derive(Debug, Clone)]
pub enum HypothesisStatus {
    /// AI generated, awaiting human review.
    Proposed,
    /// Human approved for testing.
    Approved,
    /// Experiment currently running.
    Testing,
    /// Evidence supports the hypothesis.
    Confirmed(Outcome),
    /// Evidence contradicts the hypothesis.
    Falsified(Outcome),
    /// Hypothesis refined into a new one.
    Refined(HypothesisId),
}

/// Tier: T3 — A designed test for a hypothesis.
#[derive(Debug, Clone)]
pub struct Experiment {
    /// The hypothesis being tested.
    pub hypothesis_id: HypothesisId,
    /// How the test will be conducted.
    pub method: ExperimentMethod,
    /// What we expect to observe if the hypothesis is true.
    pub expected_outcome: String,
    /// What was actually observed (filled after execution).
    pub actual_outcome: Option<Outcome>,
}

/// Tier: T2-C — The method of an experiment.
#[derive(Debug, Clone)]
pub enum ExperimentMethod {
    /// Run PV signal detection with varying thresholds.
    SignalDetection {
        drug: String,
        event: String,
        thresholds: Vec<f64>,
    },
    /// Execute code and compare output.
    CodeExecution { code: String, expected: String },
    /// Call a REST API endpoint.
    ApiCall { endpoint: String, payload: String },
    /// Check if a crate compiles.
    CompileCheck { crate_name: String },
    /// Run tests matching a pattern.
    TestRun { test_pattern: String },
}

/// Tier: T3 — The result of an experiment.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Whether the hypothesis was supported.
    pub supported: bool,
    /// What was observed.
    pub observation: String,
    /// Quantitative result if applicable.
    pub metric: Option<f64>,
    /// Confidence in the outcome itself.
    pub confidence: f64,
    /// How long the experiment took (milliseconds).
    pub duration_ms: u64,
}

/// Tier: T3 — Knowledge extracted from an experiment.
///
/// A learning is the integration of a hypothesis with its outcome.
/// It captures what was expected, what happened, and what we now know.
#[derive(Debug, Clone)]
pub struct Learning {
    /// Source hypothesis.
    pub hypothesis_id: HypothesisId,
    /// The hypothesis claim.
    pub claim: String,
    /// The experiment outcome.
    pub outcome: Outcome,
    /// What we learned from the discrepancy (or confirmation).
    pub insight: String,
    /// How much confidence changed.
    pub confidence_delta: f64,
    /// New hypotheses spawned from this learning.
    pub follow_up: Vec<String>,
}

// ── ID generation ──────────────────────────────────────────────────

/// Counter for generating unique hypothesis IDs.
///
/// Thread-safe via `AtomicU64`. Each call returns a new unique ID.
static NEXT_HYPOTHESIS_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Counter for generating unique experience IDs.
static NEXT_EXPERIENCE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl HypothesisId {
    /// Generate a new unique hypothesis ID.
    #[must_use]
    pub fn next() -> Self {
        Self(NEXT_HYPOTHESIS_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

impl ExperienceId {
    /// Generate a new unique experience ID.
    #[must_use]
    pub fn next() -> Self {
        Self(NEXT_EXPERIENCE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

// ── Constructors ───────────────────────────────────────────────────

impl Hypothesis {
    /// Create a new hypothesis in the Proposed state.
    #[must_use]
    pub fn new(claim: impl Into<String>, falsification: impl Into<String>) -> Self {
        Self {
            id: HypothesisId::next(),
            claim: claim.into(),
            confidence: Uncertain::prior(0.5, 0.5),
            falsification: falsification.into(),
            context: Vec::new(),
            status: HypothesisStatus::Proposed,
        }
    }

    /// Add context that led to this hypothesis.
    #[must_use]
    pub fn with_context(mut self, ctx: Vec<String>) -> Self {
        self.context = ctx;
        self
    }

    /// Set initial confidence.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = Uncertain::prior(confidence, confidence);
        self
    }

    /// Approve the hypothesis for testing.
    pub fn approve(&mut self) {
        self.status = HypothesisStatus::Approved;
    }

    /// Mark as currently being tested.
    pub fn start_testing(&mut self) {
        self.status = HypothesisStatus::Testing;
    }

    /// Record the outcome and update status.
    pub fn record_outcome(&mut self, outcome: Outcome) {
        if outcome.supported {
            self.confidence.update(outcome.confidence);
            self.status = HypothesisStatus::Confirmed(outcome);
        } else {
            self.confidence.update(1.0 - outcome.confidence);
            self.status = HypothesisStatus::Falsified(outcome);
        }
    }

    /// Whether the hypothesis is in a terminal state.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            HypothesisStatus::Confirmed(_)
                | HypothesisStatus::Falsified(_)
                | HypothesisStatus::Refined(_)
        )
    }

    /// Create an experiment to test this hypothesis.
    #[must_use]
    pub fn design_experiment(&self, method: ExperimentMethod) -> Experiment {
        Experiment {
            hypothesis_id: self.id,
            method,
            expected_outcome: self.claim.clone(),
            actual_outcome: None,
        }
    }
}

impl Outcome {
    /// Create a successful outcome.
    #[must_use]
    pub fn success(observation: impl Into<String>, confidence: f64, duration_ms: u64) -> Self {
        Self {
            supported: true,
            observation: observation.into(),
            metric: None,
            confidence: confidence.clamp(0.0, 1.0),
            duration_ms,
        }
    }

    /// Create a failed outcome.
    #[must_use]
    pub fn failure(observation: impl Into<String>, confidence: f64, duration_ms: u64) -> Self {
        Self {
            supported: false,
            observation: observation.into(),
            metric: None,
            confidence: confidence.clamp(0.0, 1.0),
            duration_ms,
        }
    }

    /// Attach a quantitative metric.
    #[must_use]
    pub fn with_metric(mut self, metric: f64) -> Self {
        self.metric = Some(metric);
        self
    }
}

impl Learning {
    /// Create a learning from a hypothesis and its outcome.
    #[must_use]
    pub fn integrate(hypothesis: &Hypothesis, outcome: &Outcome) -> Self {
        let insight = if outcome.supported {
            format!(
                "Confirmed: '{}'. Observation: {}",
                hypothesis.claim, outcome.observation
            )
        } else {
            format!(
                "Falsified: '{}'. Instead observed: {}",
                hypothesis.claim, outcome.observation
            )
        };

        let confidence_delta = if outcome.supported {
            outcome.confidence * 0.1 // Confirmation modestly increases confidence
        } else {
            -(outcome.confidence * 0.2) // Falsification has bigger impact
        };

        Self {
            hypothesis_id: hypothesis.id,
            claim: hypothesis.claim.clone(),
            outcome: outcome.clone(),
            insight,
            confidence_delta,
            follow_up: Vec::new(),
        }
    }

    /// Add follow-up hypotheses spawned by this learning.
    #[must_use]
    pub fn with_follow_up(mut self, follow_up: Vec<String>) -> Self {
        self.follow_up = follow_up;
        self
    }
}

// ── Display ────────────────────────────────────────────────────────

impl fmt::Display for HypothesisId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "H-{:04}", self.0)
    }
}

impl fmt::Display for ExperienceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E-{:04}", self.0)
    }
}

impl fmt::Display for HypothesisStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proposed => write!(f, "Proposed"),
            Self::Approved => write!(f, "Approved"),
            Self::Testing => write!(f, "Testing"),
            Self::Confirmed(_) => write!(f, "Confirmed"),
            Self::Falsified(_) => write!(f, "Falsified"),
            Self::Refined(id) => write!(f, "Refined -> {id}"),
        }
    }
}

impl fmt::Display for ExperimentMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SignalDetection { drug, event, .. } => {
                write!(f, "Signal({drug}/{event})")
            }
            Self::CodeExecution { .. } => write!(f, "Code"),
            Self::ApiCall { endpoint, .. } => write!(f, "API({endpoint})"),
            Self::CompileCheck { crate_name } => write!(f, "Compile({crate_name})"),
            Self::TestRun { test_pattern } => write!(f, "Test({test_pattern})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hypothesis_lifecycle() {
        let mut h = Hypothesis::new(
            "PRR >= 2.0 is sufficient for signal detection",
            "Find a known signal with PRR < 2.0",
        );
        assert!(matches!(h.status, HypothesisStatus::Proposed));
        assert!(!h.is_complete());

        h.approve();
        assert!(matches!(h.status, HypothesisStatus::Approved));

        h.start_testing();
        assert!(matches!(h.status, HypothesisStatus::Testing));

        let outcome = Outcome::success("PRR = 2.3 detected signal", 0.85, 150);
        h.record_outcome(outcome);
        assert!(h.is_complete());
        assert!(matches!(h.status, HypothesisStatus::Confirmed(_)));
    }

    #[test]
    fn test_hypothesis_falsification() {
        let mut h = Hypothesis::new("All cats are black", "Find a non-black cat");
        h.approve();
        h.start_testing();

        let outcome = Outcome::failure("Found an orange cat", 0.95, 50);
        h.record_outcome(outcome);
        assert!(matches!(h.status, HypothesisStatus::Falsified(_)));
    }

    #[test]
    fn test_unique_ids() {
        let id1 = HypothesisId::next();
        let id2 = HypothesisId::next();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_learning_integration() {
        let h = Hypothesis::new("Signal is present", "No signal detected");
        let outcome = Outcome::success("Signal detected at PRR=2.5", 0.8, 200);
        let learning = Learning::integrate(&h, &outcome);
        assert!(learning.confidence_delta > 0.0);
        assert!(learning.insight.contains("Confirmed"));
    }

    #[test]
    fn test_learning_falsification_delta() {
        let h = Hypothesis::new("Signal is present", "No signal detected");
        let outcome = Outcome::failure("No signal at any threshold", 0.9, 300);
        let learning = Learning::integrate(&h, &outcome);
        assert!(learning.confidence_delta < 0.0);
        assert!(learning.insight.contains("Falsified"));
    }

    #[test]
    fn test_experiment_design() {
        let h = Hypothesis::new("Drug X causes event Y", "");
        let exp = h.design_experiment(ExperimentMethod::SignalDetection {
            drug: "DrugX".into(),
            event: "EventY".into(),
            thresholds: vec![1.5, 2.0, 2.5],
        });
        assert_eq!(exp.hypothesis_id, h.id);
        assert!(exp.actual_outcome.is_none());
    }

    #[test]
    fn test_outcome_with_metric() {
        let o = Outcome::success("PRR=2.5", 0.8, 100).with_metric(2.5);
        assert_eq!(o.metric, Some(2.5));
    }

    #[test]
    fn test_display_formatting() {
        let id = HypothesisId(42);
        assert_eq!(format!("{id}"), "H-0042");

        let eid = ExperienceId(7);
        assert_eq!(format!("{eid}"), "E-0007");
    }
}
