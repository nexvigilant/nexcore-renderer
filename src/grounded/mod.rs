//! GROUNDED Loop — the core reasoning cycle.
//!
//! Implements the reason → test → observe → integrate → persist cycle
//! from the GROUNDED specification. Each iteration produces a `Learning`
//! that is persisted to the `ExperienceStore`.
//!
//! ```text
//! loop {
//!     let hypothesis = reason(context);
//!     let experiment = design_test(hypothesis);
//!     let outcome = execute(experiment);
//!     let learning = integrate(hypothesis, outcome);
//!     context.update(learning);
//! }
//! ```
//!
//! ## Tier Classification
//!
//! - `GroundedLoop`: T3 (domain orchestrator)
//! - `GroundedContext`: T2-C (composed state)

pub mod confidence;
pub mod experience;
pub mod hypothesis;

pub use confidence::{ConfidenceSource, Uncertain};
pub use experience::{Experience, ExperienceStore};
pub use hypothesis::{
    ExperienceId, Experiment, ExperimentMethod, Hypothesis, HypothesisId, HypothesisStatus,
    Learning, Outcome,
};

/// Tier: T2-C — Context accumulated across GROUNDED cycles.
#[derive(Debug, Default)]
pub struct GroundedContext {
    /// Key insights learned so far.
    pub insights: Vec<String>,
    /// Current overall confidence in the system.
    pub overall_confidence: f64,
    /// Number of cycles completed.
    pub cycle_count: u64,
}

impl GroundedContext {
    /// Update context with a new learning.
    pub fn update(&mut self, learning: &Learning) {
        self.insights.push(learning.insight.clone());
        // Keep only the most recent 100 insights
        if self.insights.len() > 100 {
            let drain_count = self.insights.len() - 100;
            self.insights.drain(..drain_count);
        }
        self.overall_confidence =
            (self.overall_confidence + learning.confidence_delta).clamp(0.0, 1.0);
    }
}

/// Tier: T3 — The GROUNDED loop orchestrator.
///
/// Manages the hypothesis queue, experience store, and cycle counter.
/// Each `iterate()` call runs one full cycle: test → observe → integrate.
#[derive(Debug, Default)]
pub struct GroundedLoop {
    /// Accumulated context.
    pub context: GroundedContext,
    /// Persistent experience store.
    pub experience_store: ExperienceStore,
    /// Queue of hypotheses awaiting testing.
    pub hypothesis_queue: Vec<Hypothesis>,
    /// Currently active hypothesis (being tested).
    pub active_hypothesis: Option<Hypothesis>,
}

impl GroundedLoop {
    /// Create a new GROUNDED loop.
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: GroundedContext {
                overall_confidence: 0.5, // Start with moderate prior
                ..Default::default()
            },
            experience_store: ExperienceStore::new(),
            hypothesis_queue: Vec::new(),
            active_hypothesis: None,
        }
    }

    /// Current cycle count.
    #[must_use]
    pub fn cycle_count(&self) -> u64 {
        self.context.cycle_count
    }

    /// Current overall confidence.
    #[must_use]
    pub fn confidence(&self) -> f64 {
        self.context.overall_confidence
    }

    /// Number of queued hypotheses.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.hypothesis_queue.len()
    }

    /// Total learnings recorded.
    #[must_use]
    pub fn learning_count(&self) -> usize {
        self.experience_store.count()
    }

    /// Propose a new hypothesis (adds to queue).
    pub fn propose(&mut self, hypothesis: Hypothesis) {
        tracing::info!(
            "Hypothesis proposed: {} - {}",
            hypothesis.id,
            hypothesis.claim
        );
        self.hypothesis_queue.push(hypothesis);
    }

    /// Approve a hypothesis by ID (moves from Proposed to Approved).
    pub fn approve(&mut self, id: HypothesisId) -> bool {
        if let Some(h) = self.hypothesis_queue.iter_mut().find(|h| h.id == id) {
            h.approve();
            true
        } else {
            false
        }
    }

    /// Start testing the next approved hypothesis.
    ///
    /// Returns the hypothesis ID if one was found and started.
    pub fn start_next(&mut self) -> Option<HypothesisId> {
        let pos = self
            .hypothesis_queue
            .iter()
            .position(|h| matches!(h.status, HypothesisStatus::Approved));
        let pos = pos?;
        let mut hypothesis = self.hypothesis_queue.remove(pos);
        hypothesis.start_testing();
        let id = hypothesis.id;
        self.active_hypothesis = Some(hypothesis);
        Some(id)
    }

    /// Complete the current experiment with an outcome.
    ///
    /// This is the core GROUNDED integration step:
    /// 1. Record outcome on hypothesis
    /// 2. Create learning
    /// 3. Persist experience
    /// 4. Update context
    /// 5. Increment cycle
    pub fn complete(&mut self, outcome: Outcome) -> Option<Learning> {
        let mut hypothesis = self.active_hypothesis.take()?;
        hypothesis.record_outcome(outcome.clone());

        let learning = Learning::integrate(&hypothesis, &outcome);

        // Persist the experience
        let experience = Experience::new(
            hypothesis.id,
            self.context.insights.last().cloned().unwrap_or_default(),
            &hypothesis.claim,
            outcome,
            &learning.insight,
            learning.confidence_delta,
        );
        self.experience_store.record(experience);

        // Update context
        self.context.update(&learning);
        self.context.cycle_count += 1;

        tracing::info!(
            "GROUNDED cycle {} complete: {} (confidence: {:.0}%)",
            self.context.cycle_count,
            hypothesis.id,
            self.context.overall_confidence * 100.0,
        );

        Some(learning)
    }

    /// Get the experience store for querying.
    #[must_use]
    pub fn experiences(&self) -> &ExperienceStore {
        &self.experience_store
    }

    /// Read-only view of the hypothesis queue.
    #[must_use]
    pub fn queue(&self) -> &[Hypothesis] {
        &self.hypothesis_queue
    }

    /// Read-only reference to the experience store.
    #[must_use]
    pub fn store(&self) -> &ExperienceStore {
        &self.experience_store
    }

    /// Claim text of the currently active hypothesis (if any).
    #[must_use]
    pub fn active_claim(&self) -> Option<String> {
        self.active_hypothesis.as_ref().map(|h| h.claim.clone())
    }

    /// Integrate an external learning into the GROUNDED context.
    ///
    /// Used when a learning arrives from outside the loop (e.g., via
    /// `Message::IntegrateLearning` from the UI or API bridge) rather
    /// than through the internal `complete()` cycle.
    pub fn integrate_learning(&mut self, learning: Learning) {
        self.context.update(&learning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_grounded_cycle() {
        let mut grounded = GroundedLoop::new();
        assert_eq!(grounded.cycle_count(), 0);

        // Propose
        let h = Hypothesis::new("PRR >= 2.0 detects signals", "Known signal with PRR < 2.0");
        let hid = h.id;
        grounded.propose(h);
        assert_eq!(grounded.queue_len(), 1);

        // Approve
        assert!(grounded.approve(hid));

        // Start testing
        let started = grounded.start_next();
        assert_eq!(started, Some(hid));
        assert_eq!(grounded.queue_len(), 0);

        // Complete with outcome
        let outcome = Outcome::success("PRR=2.3 detected known signal", 0.85, 150);
        let learning = grounded.complete(outcome);
        assert!(learning.is_some());

        // Verify cycle completed
        assert_eq!(grounded.cycle_count(), 1);
        assert_eq!(grounded.learning_count(), 1);
    }

    #[test]
    fn test_multiple_cycles() {
        let mut grounded = GroundedLoop::new();
        let initial_confidence = grounded.confidence();

        for i in 0..5 {
            let h = Hypothesis::new(format!("Hypothesis {i}"), "falsification");
            let hid = h.id;
            grounded.propose(h);
            grounded.approve(hid);
            grounded.start_next();
            grounded.complete(Outcome::success("ok", 0.8, 100));
        }

        assert_eq!(grounded.cycle_count(), 5);
        assert_eq!(grounded.learning_count(), 5);
        // Confidence should have changed from repeated positive outcomes
        assert_ne!(grounded.confidence(), initial_confidence);
    }

    #[test]
    fn test_queue_accessor() {
        let mut grounded = GroundedLoop::new();
        assert!(grounded.queue().is_empty());
        let h = Hypothesis::new("test claim", "falsification");
        grounded.propose(h);
        assert_eq!(grounded.queue().len(), 1);
    }

    #[test]
    fn test_store_accessor() {
        let grounded = GroundedLoop::new();
        assert_eq!(grounded.store().count(), 0);
    }

    #[test]
    fn test_active_claim_accessor() {
        let mut grounded = GroundedLoop::new();
        assert!(grounded.active_claim().is_none());
        let h = Hypothesis::new("active test", "falsification");
        let hid = h.id;
        grounded.propose(h);
        grounded.approve(hid);
        grounded.start_next();
        assert_eq!(grounded.active_claim().as_deref(), Some("active test"));
    }

    #[test]
    fn test_context_insight_capping() {
        let mut ctx = GroundedContext::default();
        for i in 0..150 {
            let learning = Learning {
                hypothesis_id: HypothesisId::next(),
                claim: format!("claim {i}"),
                outcome: Outcome::success("ok", 0.5, 0),
                insight: format!("insight {i}"),
                confidence_delta: 0.01,
                follow_up: vec![],
            };
            ctx.update(&learning);
        }
        // Should be capped at 100
        assert_eq!(ctx.insights.len(), 100);
    }
}
