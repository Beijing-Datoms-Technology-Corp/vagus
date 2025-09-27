# Code generation from portable specifications
# Generates EVM Solidity, CosmWasm Rust, and shared types

import os
import yaml
from pathlib import Path
from typing import Dict, Any, List, Optional


class VagusCodeGenerator:
    """Generate code from portable specifications"""

    def __init__(self, spec_dir: str = None, templates_dir: str = "templates"):
        if spec_dir is None:
            # Default to sibling directory of planner
            planner_dir = Path(__file__).parent.parent
            spec_dir = planner_dir.parent / "spec"

        self.spec_dir = Path(spec_dir)
        self.templates_dir = Path(__file__).parent / templates_dir
        self.spec_data = self._load_specs()

    def _load_specs(self) -> Dict[str, Any]:
        """Load all specification files"""
        specs = {}
        for spec_file in ["types.yml", "events.yml", "invariants.yml", "errors.yml"]:
            spec_path = self.spec_dir / spec_file
            if spec_path.exists():
                with open(spec_path, 'r') as f:
                    specs[spec_file.replace('.yml', '')] = yaml.safe_load(f)
        return specs

    def generate_evm_types(self, output_dir: str = "../contracts/src/core"):
        """Generate Solidity types and errors"""
        output_path = Path(output_dir) / "GeneratedTypes.sol"

        content = self._generate_solidity_types()

        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, 'w') as f:
            f.write(content)

        print(f"Generated EVM types: {output_path}")

    def generate_evm_events(self, output_dir: str = "../contracts/src/core"):
        """Generate Solidity events"""
        output_path = Path(output_dir) / "GeneratedEvents.sol"

        content = self._generate_solidity_events()

        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, 'w') as f:
            f.write(content)

        print(f"Generated EVM events: {output_path}")

    def _generate_solidity_types(self) -> str:
        """Generate Solidity type definitions"""
        lines = []

        # Header
        lines.append("// SPDX-License-Identifier: Apache-2.0")
        lines.append("pragma solidity ^0.8.24;")
        lines.append("")
        lines.append("// Auto-generated from spec/types.yml")
        lines.append("// DO NOT EDIT MANUALLY")
        lines.append("")

        # Enums
        enums = self.spec_data.get('types', {}).get('enums', {})
        for enum_name, enum_data in enums.items():
            lines.append(f"enum {enum_name} {{")
            values = enum_data.get('values', {})
            value_lines = []
            for value_name in values.keys():
                value_lines.append(f"    {value_name}")
            lines.append(",\n".join(value_lines))
            lines.append("}")
            lines.append("")

        # Structs
        structs = self.spec_data.get('types', {}).get('structs', {})
        for struct_name, struct_data in structs.items():
            lines.append(f"struct {struct_name} {{")
            fields = struct_data.get('fields', {})
            for field_name, field_info in fields.items():
                field_type = field_info.get('type', 'uint256')
                # Map to Solidity types
                if field_type == 'uint256':
                    sol_type = 'uint256'
                elif field_type == 'bytes32':
                    sol_type = 'bytes32'
                elif field_type == 'bytes':
                    sol_type = 'bytes'
                elif field_type == 'address':
                    sol_type = 'address'
                elif field_type == 'bool':
                    sol_type = 'bool'
                elif field_type == 'uint8':
                    sol_type = 'uint8'
                else:
                    sol_type = field_type

                lines.append(f"    {sol_type} {field_name};")
            lines.append("}")
            lines.append("")

        # Constants
        constants = self.spec_data.get('types', {}).get('constants', {})
        for const_name, const_data in constants.items():
            const_type = const_data.get('type', 'uint256')
            const_value = const_data.get('value', 0)
            lines.append(f"{const_type} constant {const_name} = {const_value};")
        lines.append("")

        # Errors
        errors = self.spec_data.get('errors', {}).get('errors', {})
        for error_name, error_data in errors.items():
            evm_def = error_data.get('evm', f'error {error_name}();')
            lines.append(evm_def)

        return "\n".join(lines)

    def _generate_solidity_events(self) -> str:
        """Generate Solidity event definitions"""
        lines = []

        # Header
        lines.append("// SPDX-License-Identifier: Apache-2.0")
        lines.append("pragma solidity ^0.8.24;")
        lines.append("")
        lines.append("// Auto-generated from spec/events.yml")
        lines.append("// DO NOT EDIT MANUALLY")
        lines.append("")

        # Events
        events = self.spec_data.get('events', {}).get('events', {})
        for event_name, event_data in events.items():
            keys = event_data.get('keys', {})
            indexed_keys = []
            data_keys = []

            for key_name, key_info in keys.items():
                indexed = key_info.get('indexed', False)
                key_type = key_info.get('type', 'uint256')

                # Map to Solidity types
                if key_type == 'uint256':
                    sol_type = 'uint256'
                elif key_type == 'bytes32':
                    sol_type = 'bytes32'
                elif key_type == 'address':
                    sol_type = 'address'
                elif key_type == 'uint8':
                    sol_type = 'uint8'
                elif key_type == 'string':
                    sol_type = 'string'
                else:
                    sol_type = key_type

                if indexed:
                    indexed_keys.append(f"{sol_type} indexed {key_name}")
                else:
                    data_keys.append(f"{sol_type} {key_name}")

            # Combine indexed and data keys
            all_keys = indexed_keys + data_keys
            if all_keys:
                lines.append(f"event {event_name}({', '.join(all_keys)});")
            else:
                lines.append(f"event {event_name}();")
            lines.append("")

        return "\n".join(lines)

    def generate_cosmwasm_types(self, output_dir: str = "../wasm-contracts/cosmwasm/packages/vagus-spec/src"):
        """Generate CosmWasm Rust types"""
        output_path = Path(output_dir) / "lib.rs"

        content = self._generate_cosmwasm_types()

        output_path.parent.mkdir(parents=True, exist_ok=True)
        with open(output_path, 'w') as f:
            f.write(content)

        print(f"Generated CosmWasm types: {output_path}")

    def _generate_cosmwasm_types(self) -> str:
        """Generate CosmWasm Rust type definitions"""
        lines = []

        # Header
        lines.append("//! Auto-generated from spec/types.yml")
        lines.append("//! DO NOT EDIT MANUALLY")
        lines.append("")
        lines.append("use cosmwasm_schema::cw_serde;")
        lines.append("use cosmwasm_std::{Addr, Binary, Uint256};")
        lines.append("use thiserror::Error;")
        lines.append("")

        # Enums
        enums = self.spec_data.get('types', {}).get('enums', {})
        for enum_name, enum_data in enums.items():
            lines.append("#[cw_serde]")
            lines.append(f"pub enum {enum_name} {{")
            values = enum_data.get('values', {})
            for value_name in values.keys():
                lines.append(f"    {value_name},")
            lines.append("}")
            lines.append("")

        # Structs
        structs = self.spec_data.get('types', {}).get('structs', {})
        for struct_name, struct_data in structs.items():
            lines.append("#[cw_serde]")
            lines.append(f"pub struct {struct_name} {{")
            fields = struct_data.get('fields', {})
            for field_name, field_info in fields.items():
                rust_type = self._map_rust_type(field_info.get('type', 'Uint256'))
                lines.append(f"    pub {field_name}: {rust_type},")
            lines.append("}")
            lines.append("")

        # Constants
        constants = self.spec_data.get('types', {}).get('constants', {})
        for const_name, const_data in constants.items():
            const_value = const_data.get('value', 0)
            lines.append(f"pub const {const_name}: u64 = {const_value};")
        lines.append("")

        # Errors
        errors = self.spec_data.get('errors', {}).get('errors', {})
        lines.append("#[derive(Error, Debug)]")
        lines.append("pub enum VagusError {")
        lines.append("    #[error(\"{0}\")]")
        lines.append("    Std(#[from] cosmwasm_std::StdError),")
        for error_name, error_data in errors.items():
            lines.append(f"    #[error(\"{error_data.get('description', error_name)}\")]")
            lines.append(f"    {error_name},")
        lines.append("}")

        return "\n".join(lines)

    def _map_rust_type(self, spec_type: str) -> str:
        """Map spec type to Rust type"""
        type_map = {
            'uint256': 'Uint256',
            'bytes32': 'Binary',  # 32 bytes
            'bytes': 'Binary',
            'address': 'String',  # bech32
            'bool': 'bool',
            'uint8': 'u8'
        }
        return type_map.get(spec_type, spec_type)

    def generate_all(self):
        """Generate all code from specifications"""
        self.generate_evm_types()
        self.generate_evm_events()
        self.generate_cosmwasm_types()
        print("Code generation completed successfully!")


def main():
    """Main entry point for code generation"""
    generator = VagusCodeGenerator()
    generator.generate_all()


if __name__ == "__main__":
    main()
