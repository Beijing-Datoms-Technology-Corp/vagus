# Schema loading and validation

import yaml
import os
from typing import Dict, Any, List, Optional, Tuple
from pydantic import BaseModel, Field, validator


class ParameterSchema(BaseModel):
    type: str
    unit: str
    min: float
    max: float
    brakeable: bool


class ActionSchema(BaseModel):
    description: str
    parameters: Dict[str, ParameterSchema]


class StateScaling(BaseModel):
    speed: float
    force: float


class StatePolicy(BaseModel):
    description: str
    scaling: StateScaling
    restrictions: List[str]


class PolicySchema(BaseModel):
    states: Dict[str, StatePolicy]


class SchemaManager:
    """Manages loading and validation of action schemas and policies"""

    def __init__(self, schema_dir: str = None):
        if schema_dir is None:
            # Default to sibling directory of this package
            import os
            package_dir = os.path.dirname(os.path.abspath(__file__))
            # Go up two levels: vagus_planner -> planner -> vagus
            planner_dir = os.path.dirname(package_dir)
            project_root = os.path.dirname(planner_dir)
            schema_dir = os.path.join(project_root, "schemas")

        self.schema_dir = os.path.abspath(schema_dir)
        self.actions: Dict[str, ActionSchema] = {}
        self.policy: Optional[PolicySchema] = None
        self._load_schemas()

    def _load_schemas(self):
        """Load all schemas from the schema directory"""
        # Load mechanical arm schemas
        arm_dir = os.path.join(self.schema_dir, "mechanical_arm")

        # Load actions
        actions_file = os.path.join(arm_dir, "actions.yaml")
        if os.path.exists(actions_file):
            with open(actions_file, 'r') as f:
                data = yaml.safe_load(f)
                for action_name, action_data in data.get('actions', {}).items():
                    self.actions[action_name] = ActionSchema(**action_data)

        # Load policy
        policy_file = os.path.join(arm_dir, "policy.yaml")
        if os.path.exists(policy_file):
            with open(policy_file, 'r') as f:
                data = yaml.safe_load(f)
                self.policy = PolicySchema(**data)

    def get_action_schema(self, action_name: str) -> Optional[ActionSchema]:
        """Get schema for a specific action"""
        return self.actions.get(action_name)

    def get_parameter_schema(self, action_name: str, param_name: str) -> Optional[ParameterSchema]:
        """Get schema for a specific parameter"""
        action = self.get_action_schema(action_name)
        if action:
            return action.parameters.get(param_name)
        return None

    def validate_parameter(self, action_name: str, param_name: str, value: float) -> Tuple[bool, Optional[str]]:
        """Validate a parameter value against its schema"""
        param_schema = self.get_parameter_schema(action_name, param_name)
        if not param_schema:
            return False, f"Parameter {param_name} not found in action {action_name}"

        if value < param_schema.min:
            return False, f"Value {value} below minimum {param_schema.min}"

        if value > param_schema.max:
            return False, f"Value {value} above maximum {param_schema.max}"

        return True, None

    def get_scaling_factors(self, ans_state: str) -> Optional[StateScaling]:
        """Get scaling factors for a given ANS state"""
        if self.policy and ans_state in self.policy.states:
            return self.policy.states[ans_state].scaling
        return None

    def compute_scaled_limits_hash(self, action_name: str, ans_state: str) -> str:
        """Compute scaled limits hash for an action and ANS state"""
        # This is a simplified implementation
        # In production, this would create a proper hash of scaled parameter limits
        scaling = self.get_scaling_factors(ans_state)
        if not scaling:
            return "0x" + "0" * 64  # Zero hash

        # Simple hash based on action and scaling factors
        import hashlib
        data = f"{action_name}:{scaling.speed}:{scaling.force}"
        hash_obj = hashlib.sha256(data.encode())
        return "0x" + hash_obj.hexdigest()


# Global schema manager instance
schema_manager = SchemaManager()
