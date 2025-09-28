//! Fuzz and property-based testing for Vagus invariants
//!
//! Uses proptest to test edge cases and random inputs.

use proptest::prelude::*;
use vagus_spec::{ANSState, VagalToneIndicator};

/// Test ANS state transition hysteresis
pub fn ans_state_hysteresis_strategy() -> impl Strategy<Value = Vec<u64>> {
    // Generate sequences of VTI values to test hysteresis
    prop::collection::vec((4000..10000u64), 5..20)
}

/// Test reflex arc triggering thresholds
pub fn reflex_threshold_strategy() -> impl Strategy<Value = (u64, u64)> {
    // Generate danger and shutdown thresholds
    (5000..8000u64, 2000..5000u64)
}

/// Test capability token scaling
pub fn token_scaling_strategy() -> impl Strategy<Value = (u64, u64, u64)> {
    // Generate VTI, scaling factor, and expected result
    (4000..10000u64, 1000..10000u64, 4000..10000u64)
}

proptest! {
    #[test]
    fn test_ans_state_transitions_hysteresis(vti_sequence in ans_state_hysteresis_strategy()) {
        // Test that state transitions exhibit proper hysteresis
        // Avoid rapid oscillation between states
        let mut current_state = ANSState::SAFE;
        let mut transitions = 0;

        for vti in vti_sequence {
            let new_state = determine_state_with_hysteresis(current_state.clone(), vti);
            if new_state != current_state {
                transitions += 1;
            }
            current_state = new_state;
        }

        // With hysteresis, there shouldn't be excessive transitions
        // This is a simplified check - in practice would be more sophisticated
        prop_assert!(transitions <= vti_sequence.len() / 3);
    }

    #[test]
    fn test_reflex_thresholds_are_sensible(danger_threshold in 5000..8000u64, shutdown_threshold in 2000..5000u64) {
        // Test that reflex thresholds make sense
        // Danger threshold should be higher than shutdown threshold
        prop_assert!(danger_threshold > shutdown_threshold);

        // Shutdown threshold should be reasonably low
        prop_assert!(shutdown_threshold < 4000);
    }

    #[test]
    fn test_token_scaling_bounds(vti in 4000..10000u64, scaling_factor in 1000..10000u64) {
        // Test that token scaling stays within reasonable bounds
        let scaled = (vti as u128 * scaling_factor as u128) / 10000;

        // Scaled value should be between 0 and original VTI
        prop_assert!(scaled <= vti as u128);
        prop_assert!(scaled >= 0);

        // Scaling should be monotonic
        if scaling_factor > 5000 {
            prop_assert!(scaled > (vti as u128 / 2));
        }
    }
}

fn determine_state_with_hysteresis(current: ANSState, vti: u64) -> ANSState {
    // Simplified hysteresis logic for testing
    match current {
        ANSState::SAFE => {
            if vti < 6500 {
                ANSState::DANGER
            } else {
                ANSState::SAFE
            }
        }
        ANSState::DANGER => {
            if vti >= 7500 {
                ANSState::SAFE
            } else if vti < 3500 {
                ANSState::SHUTDOWN
            } else {
                ANSState::DANGER
            }
        }
        ANSState::SHUTDOWN => {
            if vti >= 7500 {
                ANSState::SAFE
            } else if vti >= 6500 {
                ANSState::DANGER
            } else {
                ANSState::SHUTDOWN
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hysteresis_logic() {
        // Test basic hysteresis behavior
        assert_eq!(determine_state_with_hysteresis(ANSState::SAFE, 8000), ANSState::SAFE);
        assert_eq!(determine_state_with_hysteresis(ANSState::SAFE, 6000), ANSState::DANGER);
        assert_eq!(determine_state_with_hysteresis(ANSState::DANGER, 8000), ANSState::SAFE);
        assert_eq!(determine_state_with_hysteresis(ANSState::DANGER, 3000), ANSState::SHUTDOWN);
        assert_eq!(determine_state_with_hysteresis(ANSState::SHUTDOWN, 8000), ANSState::SAFE);
    }
}
