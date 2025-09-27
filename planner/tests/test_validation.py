#!/usr/bin/env python3
"""Tests for intent validation with out-of-bounds parameters"""

import pytest
import sys
import os

# Add the parent directory to the path so we can import vagus_planner
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))

from vagus_planner import IntentBuilder, validate_parameter_bounds


class TestParameterValidation:
    """Test parameter validation with bounds checking"""

    def test_valid_parameters_safe_state(self):
        """Test valid parameters in SAFE state"""
        # Valid MOVE_TO parameters
        valid, error = validate_parameter_bounds("MOVE_TO", "x", 1.0, "SAFE")
        assert valid == True
        assert error is None

        valid, error = validate_parameter_bounds("MOVE_TO", "y", -1.5, "SAFE")
        assert valid == True
        assert error is None

        valid, error = validate_parameter_bounds("MOVE_TO", "z", 2.5, "SAFE")
        assert valid == True
        assert error is None

        # Valid GRASP parameters
        valid, error = validate_parameter_bounds("GRASP", "force", 50.0, "SAFE")
        assert valid == True
        assert error is None

    def test_out_of_bounds_parameters_safe_state(self):
        """Test out-of-bounds parameters in SAFE state"""
        # X coordinate too high
        valid, error = validate_parameter_bounds("MOVE_TO", "x", 3.0, "SAFE")
        assert valid == False
        assert "above scaled maximum" in error and "2.0" in error

        # X coordinate too low
        valid, error = validate_parameter_bounds("MOVE_TO", "x", -3.0, "SAFE")
        assert valid == False
        assert "below minimum -2.0" in error

        # Z coordinate too high
        valid, error = validate_parameter_bounds("MOVE_TO", "z", 4.0, "SAFE")
        assert valid == False
        assert "above scaled maximum" in error and "3.0" in error

        # Z coordinate too low
        valid, error = validate_parameter_bounds("MOVE_TO", "z", -1.0, "SAFE")
        assert valid == False
        assert "below minimum 0.0" in error

        # Force too high
        valid, error = validate_parameter_bounds("GRASP", "force", 150.0, "SAFE")
        assert valid == False
        assert "above scaled maximum" in error and "100.0" in error

        # Force too low
        valid, error = validate_parameter_bounds("GRASP", "force", 0.5, "SAFE")
        assert valid == False
        assert "below minimum 1.0" in error

    def test_brakeable_parameter_scaling_danger_state(self):
        """Test that brakeable parameters are scaled in DANGER state"""
        # In DANGER state, speed scaling is 0.6, so vMax max becomes 2.0 * 0.6 = 1.2
        valid, error = validate_parameter_bounds("MOVE_TO", "vMax", 1.5, "DANGER")
        assert valid == False
        assert "above scaled maximum 1.200" in error

        # But 1.0 should still be valid
        valid, error = validate_parameter_bounds("MOVE_TO", "vMax", 1.0, "DANGER")
        assert valid == True
        assert error is None

        # Force scaling in DANGER is 0.7, so max force becomes 100.0 * 0.7 = 70.0
        valid, error = validate_parameter_bounds("GRASP", "force", 80.0, "DANGER")
        assert valid == False
        assert "above scaled maximum 70.000" in error

    def test_brakeable_parameter_scaling_shutdown_state(self):
        """Test that brakeable parameters are scaled in SHUTDOWN state"""
        # In SHUTDOWN state, both speed and force scaling are 0.0
        valid, error = validate_parameter_bounds("MOVE_TO", "vMax", 0.1, "SHUTDOWN")
        assert valid == False
        assert "above scaled maximum 0.000" in error

        valid, error = validate_parameter_bounds("GRASP", "force", 1.0, "SHUTDOWN")
        assert valid == False
        assert "above scaled maximum 0.000" in error

    def test_unknown_action(self):
        """Test validation with unknown action"""
        valid, error = validate_parameter_bounds("UNKNOWN_ACTION", "param", 1.0, "SAFE")
        assert valid == False
        assert "not found in action" in error

    def test_unknown_parameter(self):
        """Test validation with unknown parameter"""
        valid, error = validate_parameter_bounds("MOVE_TO", "unknown_param", 1.0, "SAFE")
        assert valid == False
        assert "not found in action" in error

    def test_unknown_ans_state(self):
        """Test validation with unknown ANS state"""
        valid, error = validate_parameter_bounds("MOVE_TO", "x", 1.0, "UNKNOWN_STATE")
        assert valid == False
        assert "Unknown ANS state" in error


class TestIntentValidation:
    """Test intent validation"""

    def test_valid_intent_creation(self):
        """Test creating a valid intent"""
        planner_address = "0x742d35Cc6645C0532925a3b8dC6b6b5a1C6Bb0B5"

        intent = (IntentBuilder(1, planner_address)
                  .set_action("MOVE_TO")
                  .set_parameter("x", 1.0)
                  .set_parameter("y", 0.5)
                  .set_parameter("z", 1.5)
                  .set_parameter("vMax", 1.0)
                  .build())

        assert intent is not None
        assert intent.executor_id == 1
        assert intent.planner == planner_address

    def test_invalid_intent_parameter_out_of_bounds(self):
        """Test that intent creation fails with out-of-bounds parameters"""
        planner_address = "0x742d35Cc6645C0532925a3b8dC6b6b6b5a1C6Bb0B5"

        with pytest.raises(ValueError) as exc_info:
            (IntentBuilder(1, planner_address)
             .set_action("MOVE_TO")
             .set_parameter("x", 5.0)  # Out of bounds (max is 2.0)
             .set_parameter("y", 0.5)
             .set_parameter("z", 1.5)
             .set_parameter("vMax", 1.0)
             .build())

        assert "above maximum 2.0" in str(exc_info.value)

    def test_invalid_intent_unknown_action(self):
        """Test that intent creation fails with unknown action"""
        planner_address = "0x742d35Cc6645C0532925a3b8dC6b6b6b5a1C6Bb0B5"

        with pytest.raises(ValueError) as exc_info:
            (IntentBuilder(1, planner_address)
             .set_action("UNKNOWN_ACTION")
             .set_parameter("param", 1.0)
             .build())

        assert "Unknown action" in str(exc_info.value)


if __name__ == "__main__":
    pytest.main([__file__])
