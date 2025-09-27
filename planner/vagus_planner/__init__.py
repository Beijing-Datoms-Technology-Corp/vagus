# Vagus Planner Package

from .schemas import schema_manager, SchemaManager, ActionSchema, ParameterSchema, PolicySchema, StateScaling
from .intents import Intent, IntentBuilder, create_move_to_intent, create_grasp_intent
from .eip712 import EIP712Encoder, EIP712Domain, encode_intent_for_signing, create_typed_data
from .validate import IntentValidator, validate_intent, validate_parameter_bounds

__all__ = [
    # Schema management
    'schema_manager',
    'SchemaManager',
    'ActionSchema',
    'ParameterSchema',
    'PolicySchema',
    'StateScaling',

    # Intent creation
    'Intent',
    'IntentBuilder',
    'create_move_to_intent',
    'create_grasp_intent',

    # EIP-712 encoding
    'EIP712Encoder',
    'EIP712Domain',
    'encode_intent_for_signing',
    'create_typed_data',

    # Validation
    'IntentValidator',
    'validate_intent',
    'validate_parameter_bounds',
]

__version__ = "0.1.0"
