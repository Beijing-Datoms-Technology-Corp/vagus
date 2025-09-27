# Intent generation and validation

import time
from typing import Dict, Any, Optional, List
from dataclasses import dataclass
from .schemas import schema_manager, ParameterSchema


@dataclass
class Intent:
    """Represents an intent to execute an action"""
    executor_id: int
    action_id: str  # keccak256 hash of action name
    params: bytes   # ABI-encoded parameters
    envelope_hash: str  # keccak256 hash
    pre_state_root: str  # bytes32
    not_before: int  # timestamp
    not_after: int   # timestamp
    max_duration_ms: int
    max_energy_j: int
    planner: str     # address
    nonce: int

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for EIP-712 encoding"""
        return {
            "executorId": self.executor_id,
            "actionId": self.action_id,
            "params": self.params.hex(),
            "envelopeHash": self.envelope_hash,
            "preStateRoot": self.pre_state_root,
            "notBefore": self.not_before,
            "notAfter": self.not_after,
            "maxDurationMs": self.max_duration_ms,
            "maxEnergyJ": self.max_energy_j,
            "planner": self.planner,
            "nonce": self.nonce,
        }


class IntentBuilder:
    """Builder for creating intents with validation"""

    def __init__(self, executor_id: int, planner_address: str):
        self.executor_id = executor_id
        self.planner_address = planner_address
        self.action_name = ""
        self.parameters = {}
        self.max_duration_ms = 30000  # 30 seconds default
        self.max_energy_j = 1000      # 1kJ default
        self.validity_duration_s = 3600  # 1 hour default

    def set_action(self, action_name: str) -> 'IntentBuilder':
        """Set the action to execute"""
        self.action_name = action_name
        return self

    def set_parameter(self, name: str, value: float) -> 'IntentBuilder':
        """Set a parameter value"""
        self.parameters[name] = value
        return self

    def set_max_duration(self, duration_ms: int) -> 'IntentBuilder':
        """Set maximum duration in milliseconds"""
        self.max_duration_ms = duration_ms
        return self

    def set_max_energy(self, energy_j: int) -> 'IntentBuilder':
        """Set maximum energy in joules"""
        self.max_energy_j = energy_j
        return self

    def set_validity_duration(self, duration_s: int) -> 'IntentBuilder':
        """Set intent validity duration in seconds"""
        self.validity_duration_s = duration_s
        return self

    def validate(self) -> List[str]:
        """Validate the intent parameters against schema"""
        errors = []

        if not self.action_name:
            errors.append("Action name is required")
            return errors

        action_schema = schema_manager.get_action_schema(self.action_name)
        if not action_schema:
            errors.append(f"Unknown action: {self.action_name}")
            return errors

        # Validate each parameter
        for param_name, param_value in self.parameters.items():
            param_schema = schema_manager.get_parameter_schema(self.action_name, param_name)
            if not param_schema:
                errors.append(f"Unknown parameter: {param_name} for action {self.action_name}")
                continue

            valid, error_msg = schema_manager.validate_parameter(self.action_name, param_name, param_value)
            if not valid:
                errors.append(f"Parameter {param_name}: {error_msg}")

        return errors

    def build(self) -> Optional[Intent]:
        """Build the intent if validation passes"""
        errors = self.validate()
        if errors:
            raise ValueError(f"Intent validation failed: {errors}")

        # Generate action ID (keccak256 of action name)
        import hashlib
        action_id = "0x" + hashlib.sha256(self.action_name.encode()).hexdigest()

        # Encode parameters (simplified ABI encoding)
        params_data = self._encode_parameters()
        params_bytes = params_data.encode('utf-8')

        # Generate envelope hash (simplified)
        envelope_data = f"{self.executor_id}:{action_id}:{params_bytes.hex()}"
        envelope_hash = "0x" + hashlib.sha256(envelope_data.encode()).hexdigest()

        # Pre-state root (placeholder)
        pre_state_root = "0x" + "0" * 64

        # Timestamps
        now = int(time.time())
        not_before = now
        not_after = now + self.validity_duration_s

        return Intent(
            executor_id=self.executor_id,
            action_id=action_id,
            params=params_bytes,
            envelope_hash=envelope_hash,
            pre_state_root=pre_state_root,
            not_before=not_before,
            not_after=not_after,
            max_duration_ms=self.max_duration_ms,
            max_energy_j=self.max_energy_j,
            planner=self.planner_address,
            nonce=now,  # Use timestamp as nonce for simplicity
        )

    def _encode_parameters(self) -> str:
        """Encode parameters in a simple format (would be proper ABI encoding in production)"""
        encoded = []
        for name, value in sorted(self.parameters.items()):
            encoded.append(f"{name}:{value}")
        return ";".join(encoded)


def create_move_to_intent(executor_id: int, planner_address: str, x: float, y: float, z: float, v_max: float = 1.0) -> Intent:
    """Convenience function to create a MOVE_TO intent"""
    return (IntentBuilder(executor_id, planner_address)
            .set_action("MOVE_TO")
            .set_parameter("x", x)
            .set_parameter("y", y)
            .set_parameter("z", z)
            .set_parameter("vMax", v_max)
            .build())


def create_grasp_intent(executor_id: int, planner_address: str, force: float, duration_ms: int) -> Intent:
    """Convenience function to create a GRASP intent"""
    return (IntentBuilder(executor_id, planner_address)
            .set_action("GRASP")
            .set_parameter("force", force)
            .set_parameter("duration", duration_ms)
            .build())
