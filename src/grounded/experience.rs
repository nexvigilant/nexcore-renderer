//! Experience Store — persistent structured learnings.
//!
//! Stores `(context, hypothesis, outcome)` triples from the GROUNDED loop.
//! Each experience is a record of what was hypothesized, what was tested,
//! and what was learned.
//!
//! ## Tier Classification
//!
//! - `Experience`: T3 (domain-specific structured record)
//! - `ExperienceStore`: T3 (domain-specific collection)

use super::hypothesis::{ExperienceId, HypothesisId, Outcome};

/// Tier: T3 — A structured experience record.
///
/// Each experience captures the full GROUNDED cycle:
/// context → hypothesis → outcome → learning.
#[derive(Debug, Clone)]
pub struct Experience {
    /// Unique identifier.
    pub id: ExperienceId,
    /// When this experience was recorded (epoch millis).
    pub timestamp_ms: u64,
    /// The context that led to the hypothesis.
    pub context: String,
    /// The hypothesis that was tested.
    pub hypothesis: String,
    /// The hypothesis ID for cross-referencing.
    pub hypothesis_id: HypothesisId,
    /// The outcome of the experiment.
    pub outcome: Outcome,
    /// What was learned.
    pub learning: String,
    /// How much confidence changed.
    pub confidence_delta: f64,
}

/// Tier: T3 — In-memory store of experiences.
///
/// Future: will persist via `nexcore-brain` artifacts.
#[derive(Debug, Default)]
pub struct ExperienceStore {
    /// All recorded experiences.
    experiences: Vec<Experience>,
}

impl ExperienceStore {
    /// Create a new empty store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            experiences: Vec::new(),
        }
    }

    /// Record a new experience.
    pub fn record(&mut self, experience: Experience) {
        tracing::info!(
            "Experience recorded: {} (delta: {:.2})",
            experience.id,
            experience.confidence_delta,
        );
        self.experiences.push(experience);
    }

    /// Get all experiences.
    #[must_use]
    pub fn all(&self) -> &[Experience] {
        &self.experiences
    }

    /// Get the most recent N experiences.
    #[must_use]
    pub fn recent(&self, n: usize) -> &[Experience] {
        let start = self.experiences.len().saturating_sub(n);
        &self.experiences[start..]
    }

    /// Total number of recorded experiences.
    #[must_use]
    pub fn count(&self) -> usize {
        self.experiences.len()
    }

    /// Find experiences related to a hypothesis.
    #[must_use]
    pub fn for_hypothesis(&self, id: HypothesisId) -> Vec<&Experience> {
        self.experiences
            .iter()
            .filter(|e| e.hypothesis_id == id)
            .collect()
    }

    /// Search experiences by keyword in context, hypothesis, or learning.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&Experience> {
        let q = query.to_lowercase();
        self.experiences
            .iter()
            .filter(|e| experience_matches_query(e, &q))
            .collect()
    }

    /// Compute the average confidence delta across all experiences.
    #[must_use]
    pub fn average_confidence_delta(&self) -> f64 {
        if self.experiences.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.experiences.iter().map(|e| e.confidence_delta).sum();
        sum / self.experiences.len() as f64
    }

    /// Count of experiences where hypothesis was confirmed.
    #[must_use]
    pub fn confirmed_count(&self) -> usize {
        self.experiences
            .iter()
            .filter(|e| e.outcome.supported)
            .count()
    }

    /// Count of experiences where hypothesis was falsified.
    #[must_use]
    pub fn falsified_count(&self) -> usize {
        self.experiences
            .iter()
            .filter(|e| !e.outcome.supported)
            .count()
    }

    /// Confirmation rate (0.0 to 1.0).
    #[must_use]
    pub fn confirmation_rate(&self) -> f64 {
        if self.experiences.is_empty() {
            return 0.0;
        }
        self.confirmed_count() as f64 / self.experiences.len() as f64
    }
}

/// Check if an experience matches a search query.
fn experience_matches_query(exp: &Experience, query: &str) -> bool {
    exp.context.to_lowercase().contains(query)
        || exp.hypothesis.to_lowercase().contains(query)
        || exp.learning.to_lowercase().contains(query)
}

/// Helper to get current epoch millis without chrono dependency.
#[must_use]
pub fn epoch_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

impl Experience {
    /// Create a new experience from GROUNDED loop output.
    #[must_use]
    pub fn new(
        hypothesis_id: HypothesisId,
        context: impl Into<String>,
        hypothesis: impl Into<String>,
        outcome: Outcome,
        learning: impl Into<String>,
        confidence_delta: f64,
    ) -> Self {
        Self {
            id: ExperienceId::next(),
            timestamp_ms: epoch_millis(),
            context: context.into(),
            hypothesis: hypothesis.into(),
            hypothesis_id,
            outcome,
            learning: learning.into(),
            confidence_delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_experience(supported: bool) -> Experience {
        let outcome = if supported {
            Outcome::success("signal found", 0.8, 100)
        } else {
            Outcome::failure("no signal", 0.9, 200)
        };
        let delta = if supported { 0.1 } else { -0.2 };
        Experience::new(
            HypothesisId::next(),
            "PV context",
            "Drug X causes event Y",
            outcome,
            "Learned something",
            delta,
        )
    }

    #[test]
    fn test_store_record_and_count() {
        let mut store = ExperienceStore::new();
        assert_eq!(store.count(), 0);

        store.record(sample_experience(true));
        assert_eq!(store.count(), 1);

        store.record(sample_experience(false));
        assert_eq!(store.count(), 2);
    }

    #[test]
    fn test_recent() {
        let mut store = ExperienceStore::new();
        for _ in 0..5 {
            store.record(sample_experience(true));
        }
        assert_eq!(store.recent(3).len(), 3);
        assert_eq!(store.recent(10).len(), 5);
    }

    #[test]
    fn test_search() {
        let mut store = ExperienceStore::new();
        store.record(sample_experience(true));

        let results = store.search("Drug X");
        assert_eq!(results.len(), 1);

        let results = store.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_confirmation_rate() {
        let mut store = ExperienceStore::new();
        store.record(sample_experience(true));
        store.record(sample_experience(true));
        store.record(sample_experience(false));
        assert!((store.confirmation_rate() - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_average_confidence_delta() {
        let mut store = ExperienceStore::new();
        store.record(sample_experience(true)); // +0.1
        store.record(sample_experience(false)); // -0.2
        let avg = store.average_confidence_delta();
        assert!((avg - (-0.05)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_empty_store_defaults() {
        let store = ExperienceStore::new();
        assert!(store.average_confidence_delta().abs() < f64::EPSILON);
        assert!(store.confirmation_rate().abs() < f64::EPSILON);
    }
}
