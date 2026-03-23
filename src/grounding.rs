//! # GroundsTo implementations for nexcore-renderer types
//!
//! Connects browser rendering pipeline types to the Lex Primitiva type system.
//!
//! ## Domain Signature
//!
//! - **σ (Sequence)**: dominant -- the rendering pipeline IS a sequence
//! - **ρ (Recursion)**: DOM tree, layout tree
//! - **λ (Location)**: spatial positioning, URLs
//!
//! Note: Many renderer types live in submodules (dom, layout, paint, etc.)
//! that are tightly coupled to GPU/windowing crates. This module grounds
//! the top-level public types and the GROUNDED loop types.

use nexcore_lex_primitiva::grounding::GroundsTo;
use nexcore_lex_primitiva::primitiva::{LexPrimitiva, PrimitiveComposition};

use crate::Error;
use crate::grounded::Uncertain;
use crate::grounded::confidence::ConfidenceSource;
use crate::grounded::hypothesis::{
    Experiment, ExperimentMethod, Hypothesis, HypothesisStatus, Learning, Outcome,
};
use crate::grounded::{GroundedContext, GroundedLoop};

// ---------------------------------------------------------------------------
// T2-P: Error type
// ---------------------------------------------------------------------------

/// Error: T2-C (∂ + → + ∅ + λ), dominant ∂
///
/// Renderer errors: network, parse, layout, render, URL, I/O.
impl GroundsTo for Error {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Boundary,  // ∂ -- format/constraint violations
            LexPrimitiva::Causality, // → -- operation failures
            LexPrimitiva::Void,      // ∅ -- parse failures
            LexPrimitiva::Location,  // λ -- URL parsing
        ])
        .with_dominant(LexPrimitiva::Boundary, 0.85)
    }
}

// ---------------------------------------------------------------------------
// GROUNDED Loop types
// ---------------------------------------------------------------------------

/// HypothesisStatus: T2-P (ς + σ), dominant ς
///
/// Lifecycle status: Proposed -> Approved -> Testing -> Completed/Rejected.
impl GroundsTo for HypothesisStatus {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,    // ς -- lifecycle position
            LexPrimitiva::Sequence, // σ -- ordered transitions
        ])
        .with_dominant(LexPrimitiva::State, 0.90)
    }
}

/// ExperimentMethod: T2-P (→ + Σ), dominant →
///
/// How to test a hypothesis: Observe, Measure, Compare, Falsify.
impl GroundsTo for ExperimentMethod {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → -- testing causes outcome
            LexPrimitiva::Sum,       // Σ -- method variant
        ])
        .with_dominant(LexPrimitiva::Causality, 0.85)
    }
}

/// Experiment: T2-C (→ + ∂ + Σ + N), dominant →
///
/// A designed test with method, parameters, and success criteria.
impl GroundsTo for Experiment {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality, // → -- test → outcome
            LexPrimitiva::Boundary,  // ∂ -- success criteria
            LexPrimitiva::Sum,       // Σ -- method variant
            LexPrimitiva::Quantity,  // N -- sample count
        ])
        .with_dominant(LexPrimitiva::Causality, 0.80)
    }
}

/// Hypothesis: T3 (ρ + ς + → + ∂ + λ + N), dominant ρ
///
/// A falsifiable claim with experiments and outcomes.
/// Recursion-dominant: hypotheses can generate follow-up hypotheses.
impl GroundsTo for Hypothesis {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Recursion, // ρ -- follow-up hypotheses
            LexPrimitiva::State,     // ς -- status lifecycle
            LexPrimitiva::Causality, // → -- claim → evidence
            LexPrimitiva::Boundary,  // ∂ -- falsification criteria
            LexPrimitiva::Location,  // λ -- hypothesis ID
            LexPrimitiva::Quantity,  // N -- confidence values
        ])
        .with_dominant(LexPrimitiva::Recursion, 0.80)
    }
}

/// Outcome: T2-C (κ + N + → + ∃), dominant κ
///
/// Result of an experiment. Comparison-dominant: success/failure evaluation.
impl GroundsTo for Outcome {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison, // κ -- success/failure judgment
            LexPrimitiva::Quantity,   // N -- confidence, sample size
            LexPrimitiva::Causality,  // → -- experiment → result
            LexPrimitiva::Existence,  // ∃ -- evidence exists
        ])
        .with_dominant(LexPrimitiva::Comparison, 0.80)
    }
}

/// Learning: T2-C (→ + ς + κ + N), dominant →
///
/// Integrated learning from a GROUNDED cycle.
/// Causality-dominant: hypothesis + outcome → insight.
impl GroundsTo for Learning {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Causality,  // → -- integration produces insight
            LexPrimitiva::State,      // ς -- confidence delta
            LexPrimitiva::Comparison, // κ -- evaluation
            LexPrimitiva::Quantity,   // N -- confidence values
        ])
        .with_dominant(LexPrimitiva::Causality, 0.80)
    }
}

/// ConfidenceSource: T2-P (κ + Σ), dominant κ
///
/// Source of confidence: Empirical, Theoretical, Heuristic, etc.
impl GroundsTo for ConfidenceSource {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Comparison, // κ -- source quality ranking
            LexPrimitiva::Sum,        // Σ -- source variant
        ])
        .with_dominant(LexPrimitiva::Comparison, 0.85)
    }
}

/// Uncertain<T>: T2-P (N + ∂), dominant N
///
/// A value with confidence bounds (generic over T).
impl<T> GroundsTo for Uncertain<T> {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Quantity, // N -- value and confidence
            LexPrimitiva::Boundary, // ∂ -- confidence bounds
        ])
        .with_dominant(LexPrimitiva::Quantity, 0.85)
    }
}

/// GroundedContext: T2-C (ς + σ + N + π), dominant ς
///
/// Accumulated context across GROUNDED cycles.
impl GroundsTo for GroundedContext {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::State,       // ς -- accumulated state
            LexPrimitiva::Sequence,    // σ -- insight history
            LexPrimitiva::Quantity,    // N -- cycle count, confidence
            LexPrimitiva::Persistence, // π -- persisted insights
        ])
        .with_dominant(LexPrimitiva::State, 0.80)
    }
}

/// GroundedLoop: T3 (σ + ρ + ς + → + π + κ), dominant σ
///
/// The GROUNDED loop orchestrator. Sequence-dominant: ordered cycle execution.
impl GroundsTo for GroundedLoop {
    fn primitive_composition() -> PrimitiveComposition {
        PrimitiveComposition::new(vec![
            LexPrimitiva::Sequence,    // σ -- iterate cycle
            LexPrimitiva::Recursion,   // ρ -- follow-up hypotheses
            LexPrimitiva::State,       // ς -- context accumulation
            LexPrimitiva::Causality,   // → -- test → learn
            LexPrimitiva::Persistence, // π -- experience store
            LexPrimitiva::Comparison,  // κ -- outcome evaluation
        ])
        .with_dominant(LexPrimitiva::Sequence, 0.80)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use nexcore_lex_primitiva::tier::Tier;

    #[test]
    fn renderer_error_is_boundary_dominant() {
        assert_eq!(Error::dominant_primitive(), Some(LexPrimitiva::Boundary));
    }

    #[test]
    fn hypothesis_is_recursion_dominant() {
        assert_eq!(
            Hypothesis::dominant_primitive(),
            Some(LexPrimitiva::Recursion)
        );
        assert_eq!(Hypothesis::tier(), Tier::T3DomainSpecific);
    }

    #[test]
    fn grounded_loop_is_sequence_dominant() {
        assert_eq!(
            GroundedLoop::dominant_primitive(),
            Some(LexPrimitiva::Sequence)
        );
        assert_eq!(GroundedLoop::tier(), Tier::T3DomainSpecific);
    }

    #[test]
    fn learning_is_causality_dominant() {
        assert_eq!(
            Learning::dominant_primitive(),
            Some(LexPrimitiva::Causality)
        );
    }

    #[test]
    fn outcome_is_comparison_dominant() {
        assert_eq!(
            Outcome::dominant_primitive(),
            Some(LexPrimitiva::Comparison)
        );
    }

    #[test]
    fn uncertain_is_quantity_dominant() {
        assert_eq!(
            <crate::grounded::confidence::Uncertain<f64>>::dominant_primitive(),
            Some(LexPrimitiva::Quantity)
        );
    }

    #[test]
    fn hypothesis_status_is_state_dominant() {
        assert_eq!(
            HypothesisStatus::dominant_primitive(),
            Some(LexPrimitiva::State)
        );
    }

    #[test]
    fn grounded_context_is_state_dominant() {
        assert_eq!(
            GroundedContext::dominant_primitive(),
            Some(LexPrimitiva::State)
        );
    }
}
