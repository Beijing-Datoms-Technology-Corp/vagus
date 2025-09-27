# Intent validation logic

from typing import List, Tuple, Optional
from .intents import Intent
from .schemas import schema_manager


class IntentValidator:
    """Validates intents against schemas and policies"""

    def __init__(self):
        pass

    def validate_intent(self, intent: Intent, ans_state: str = "SAFE") -> List[str]:
        """Validate an intent comprehensively"""
        errors = []

        # Basic intent validation
        basic_errors = self._validate_basic_intent(intent)
        errors.extend(basic_errors)

        # Schema validation (if we can decode parameters)
        schema_errors = self._validate_against_schema(intent)
        errors.extend(schema_errors)

        # Policy validation based on ANS state
        policy_errors = self._validate_against_policy(intent, ans_state)
        errors.extend(policy_errors)

        return errors

    def _validate_basic_intent(self, intent: Intent) -> List[str]:
        """Basic intent validation"""
        errors = []

        # Check timestamps
        import time
        now = int(time.time())

        if intent.not_before > intent.not_after:
            errors.append("not_before cannot be after not_after")

        if intent.not_after < now:
            errors.append("Intent has already expired")

        if intent.not_before > now + 3600:  # More than 1 hour in future
            errors.append("Intent validity starts too far in the future")

        # Check resource limits
        if intent.max_duration_ms <= 0:
            errors.append("max_duration_ms must be positive")

        if intent.max_energy_j <= 0:
            errors.append("max_energy_j must be positive")

        # Check executor ID
        if intent.executor_id <= 0:
            errors.append("executor_id must be positive")

        return errors

    def _validate_against_schema(self, intent: Intent) -> List[str]:
        """Validate intent against action schemas"""
        errors = []

        # For now, we can't decode the action name from action_id easily
        # In production, you'd maintain a reverse mapping or decode from params

        # TODO: Implement parameter decoding and validation
        # This would require proper ABI decoding of the params field

        return errors

    def _validate_against_policy(self, intent: Intent, ans_state: str) -> List[str]:
        """Validate intent against current ANS policy"""
        errors = []

        scaling = schema_manager.get_scaling_factors(ans_state)
        if not scaling:
            errors.append(f"Unknown ANS state: {ans_state}")
            return errors

        # Check if intent parameters need scaling
        # In production, this would decode parameters and check against scaled limits

        # For now, just log the scaling factors that would be applied
        print(f"Applying {ans_state} scaling: speed={scaling.speed}, force={scaling.force}")

        return errors

    def validate_parameter_bounds(self, action_name: str, param_name: str, value: float, ans_state: str = "SAFE") -> Tuple[bool, Optional[str]]:
        """Validate a parameter considering ANS state scaling"""
        param_schema = schema_manager.get_parameter_schema(action_name, param_name)
        if not param_schema:
            return False, f"Parameter {param_name} not found in action {action_name}"

        if not param_schema.brakeable:
            # Non-brakeable parameters use original bounds
            return schema_manager.validate_parameter(action_name, param_name, value)

        # Brakeable parameters get scaled bounds based on ANS state
        scaling = schema_manager.get_scaling_factors(ans_state)
        if not scaling:
            return False, f"Unknown ANS state: {ans_state}"

        # Apply scaling to the maximum bounds
        if param_name.lower().find('speed') >= 0 or param_name.lower().find('v') == 0:
            scaled_max = param_schema.max * scaling.speed
        elif param_name.lower().find('force') >= 0:
            scaled_max = param_schema.max * scaling.force
        else:
            # Default scaling - use the more restrictive one
            scaled_max = param_schema.max * min(scaling.speed, scaling.force)

        # Validate against scaled bounds
        if value < param_schema.min:
            return False, f"Value {value} below minimum {param_schema.min}"

        if value > scaled_max:
            return False, f"Value {value} above scaled maximum {scaled_max:.3f} (original: {param_schema.max})"

        return True, None


# Global validator instance
validator = IntentValidator()


def validate_intent(intent: Intent, ans_state: str = "SAFE") -> List[str]:
    """Convenience function to validate an intent"""
    return validator.validate_intent(intent, ans_state)


def validate_parameter_bounds(action_name: str, param_name: str, value: float, ans_state: str = "SAFE") -> Tuple[bool, Optional[str]]:
    """Convenience function to validate parameter bounds"""
    return validator.validate_parameter_bounds(action_name, param_name, value, ans_state)
