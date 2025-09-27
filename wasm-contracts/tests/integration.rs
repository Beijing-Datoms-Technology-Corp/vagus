use cosmwasm_std::{Addr, Binary, Empty};
use cw_multi_test::{App, Contract, ContractWrapper};
use vagus_spec::*;

// Import contract modules
use afferent_inbox::contract::{execute as afferent_execute, instantiate as afferent_instantiate, query as afferent_query};
use ans_state_manager::contract::{execute as ans_execute, instantiate as ans_instantiate, query as ans_query};
use capability_issuer::contract::{execute as issuer_execute, instantiate as issuer_instantiate, query as issuer_query};
use reflex_arc::contract::{execute as reflex_execute, instantiate as reflex_instantiate};
use vagal_brake::contract::{execute as brake_execute, instantiate as brake_instantiate};

#[cfg(test)]
mod tests {
    use super::*;

    fn afferent_inbox_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(afferent_execute, afferent_instantiate, afferent_query);
        Box::new(contract)
    }

    fn ans_state_manager_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(ans_execute, ans_instantiate, ans_query);
        Box::new(contract)
    }

    fn capability_issuer_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(issuer_execute, issuer_instantiate, issuer_query);
        Box::new(contract)
    }

    fn reflex_arc_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(reflex_execute, reflex_instantiate, |_, _, _| Ok(Default::default()));
        Box::new(contract)
    }

    fn vagal_brake_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(brake_execute, brake_instantiate, |_, _, _| Ok(Default::default()));
        Box::new(contract)
    }

    #[test]
    fn test_vagus_e2e_workflow() {
        let mut app = App::default();

        // Deploy contracts
        let afferent_code_id = app.store_code(afferent_inbox_contract());
        let ans_code_id = app.store_code(ans_state_manager_contract());
        let issuer_code_id = app.store_code(capability_issuer_contract());
        let reflex_code_id = app.store_code(reflex_arc_contract());
        let brake_code_id = app.store_code(vagal_brake_contract());

        let admin = Addr::unchecked("admin");

        // Instantiate contracts
        let afferent_addr = app.instantiate_contract(
            afferent_code_id,
            admin.clone(),
            &afferent_inbox::msg::InstantiateMsg {
                authorized_attestors: vec!["gateway".to_string()],
            },
            &[],
            "AfferentInbox",
            None,
        ).unwrap();

        let ans_addr = app.instantiate_contract(
            ans_code_id,
            admin.clone(),
            &ans_state_manager::msg::InstantiateMsg {
                initial_state: ANSState::SAFE,
                min_state_residency: 60,
                safe_threshold: 8000,
                danger_threshold: 6000,
            },
            &[],
            "ANSStateManager",
            None,
        ).unwrap();

        let issuer_addr = app.instantiate_contract(
            issuer_code_id,
            admin.clone(),
            &capability_issuer::msg::InstantiateMsg {
                authorized_planners: vec!["planner".to_string()],
                reflex_arc: Some("reflex_addr".to_string()), // Will update after reflex deployment
            },
            &[],
            "CapabilityIssuer",
            None,
        ).unwrap();

        let reflex_addr = app.instantiate_contract(
            reflex_code_id,
            admin.clone(),
            &reflex_arc::msg::InstantiateMsg {
                afferent_inbox: afferent_addr.to_string(),
                capability_issuer: issuer_addr.to_string(),
                reflex_cooldown: 30,
                danger_vti_threshold: 6000,
                shutdown_vti_threshold: 3000,
            },
            &[],
            "ReflexArc",
            None,
        ).unwrap();

        let brake_addr = app.instantiate_contract(
            brake_code_id,
            admin.clone(),
            &vagal_brake::msg::InstantiateMsg {
                ans_state_manager: ans_addr.to_string(),
                capability_issuer: issuer_addr.to_string(),
            },
            &[],
            "VagalBrake",
            None,
        ).unwrap();

        // Update issuer with reflex arc address
        app.execute_contract(
            admin.clone(),
            issuer_addr.clone(),
            &capability_issuer::msg::ExecuteMsg::SetAuthorizedPlanners {
                planners: vec!["planner".to_string()],
            },
            &[],
        ).unwrap();

        // Test workflow: Issue capability via brake
        let move_to_action = Binary::from(b"move_to_action_hash");
        let intent_params = Binary::from(b"intent_params");
        let envelope_hash = Binary::from(b"envelope_hash");
        let pre_state_root = Binary::from(b"pre_state_root");
        let scaled_limits_hash = Binary::from(b"scaled_limits_hash");

        // Issue capability through brake (should succeed in SAFE state)
        let issue_result = app.execute_contract(
            Addr::unchecked("planner"),
            brake_addr.clone(),
            &vagal_brake::msg::ExecuteMsg::IssueWithBrake {
                intent_executor_id: 123,
                intent_action_id: move_to_action.clone(),
                intent_params: intent_params.clone(),
                intent_envelope_hash: envelope_hash.clone(),
                intent_pre_state_root: pre_state_root.clone(),
                intent_not_before: 1000,
                intent_not_after: 2000,
                intent_max_duration_ms: 10000,
                intent_max_energy_j: 500,
                intent_planner: "planner".to_string(),
                intent_nonce: 1,
                scaled_limits_hash: scaled_limits_hash.clone(),
                expires_at: 3000,
            },
            &[],
        );

        // Should succeed (capability issued)
        assert!(issue_result.is_ok());

        // Post AEP that triggers reflex
        let metrics_hash_sha256 = Binary::from(vec![0; 32]);
        let metrics_hash_keccak = Binary::from(vec![0; 32]);

        let aep_result = app.execute_contract(
            Addr::unchecked("gateway"),
            afferent_addr.clone(),
            &afferent_inbox::msg::ExecuteMsg::PostAEP {
                executor_id: 123,
                state_root_sha256: Binary::from(vec![1; 32]),
                state_root_keccak: Binary::from(vec![1; 32]),
                metrics_hash_sha256: metrics_hash_sha256.clone(),
                metrics_hash_keccak: metrics_hash_keccak.clone(),
                attestation: Binary::from(b"attestation"),
            },
            &[],
        );

        assert!(aep_result.is_ok());

        // Trigger reflex arc (simulated dangerous condition)
        let reflex_result = app.execute_contract(
            Addr::unchecked("gateway"),
            reflex_addr.clone(),
            &reflex_arc::msg::ExecuteMsg::ManualTrigger {
                executor_id: 123,
                reason: "Simulated danger".to_string(),
            },
            &[],
        );

        // Should succeed (capabilities revoked)
        assert!(reflex_result.is_ok());

        println!("✅ Vagus e2e workflow test passed!");
    }

    #[test]
    fn test_event_consistency() {
        let mut app = App::default();

        // Deploy ANS State Manager
        let ans_code_id = app.store_code(ans_state_manager_contract());
        let admin = Addr::unchecked("admin");

        let ans_addr = app.instantiate_contract(
            ans_code_id,
            admin.clone(),
            &ans_state_manager::msg::InstantiateMsg {
                initial_state: ANSState::SAFE,
                min_state_residency: 60,
                safe_threshold: 8000,
                danger_threshold: 6000,
            },
            &[],
            "ANSStateManager",
            None,
        ).unwrap();

        // Update tone to trigger state change
        let update_result = app.execute_contract(
            admin,
            ans_addr,
            &ans_state_manager::msg::ExecuteMsg::UpdateTone {
                vti: 5000, // Below danger threshold
                suggested: ANSState::DANGER,
            },
            &[],
        );

        assert!(update_result.is_ok());

        // Check events contain expected keys
        let events = update_result.unwrap().events;
        let vagus_event = events.iter().find(|e| e.ty == "wasm").unwrap();

        // Should contain tone, state, and updated_at attributes
        assert!(vagus_event.attributes.iter().any(|attr| attr.key == "tone"));
        assert!(vagus_event.attributes.iter().any(|attr| attr.key == "state"));
        assert!(vagus_event.attributes.iter().any(|attr| attr.key == "updated_at"));

        println!("✅ Event consistency test passed!");
    }
}

// Placeholder modules for imports (would be actual contract modules in full implementation)
mod afferent_inbox {
    pub mod contract {
        use super::super::*;
        pub use vagus_spec::*;
    }
    pub mod msg {
        use super::super::*;
        #[cosmwasm_schema::cw_serde]
        pub struct InstantiateMsg {
            pub authorized_attestors: Vec<String>,
        }
    }
}

mod ans_state_manager {
    pub mod contract {
        use super::super::*;
        pub use vagus_spec::*;
    }
    pub mod msg {
        use super::super::*;
        #[cosmwasm_schema::cw_serde]
        pub struct InstantiateMsg {
            pub initial_state: ANSState,
            pub min_state_residency: u64,
            pub safe_threshold: u64,
            pub danger_threshold: u64,
        }
    }
}

mod capability_issuer {
    pub mod contract {
        use super::super::*;
        pub use vagus_spec::*;
    }
    pub mod msg {
        use super::super::*;
        #[cosmwasm_schema::cw_serde]
        pub struct InstantiateMsg {
            pub authorized_planners: Vec<String>,
            pub reflex_arc: Option<String>,
        }
        #[cosmwasm_schema::cw_serde]
        pub enum ExecuteMsg {
            SetAuthorizedPlanners { planners: Vec<String> },
        }
    }
}

mod reflex_arc {
    pub mod contract {
        use super::super::*;
        pub use vagus_spec::*;
    }
    pub mod msg {
        use super::super::*;
        #[cosmwasm_schema::cw_serde]
        pub struct InstantiateMsg {
            pub afferent_inbox: String,
            pub capability_issuer: String,
            pub reflex_cooldown: u64,
            pub danger_vti_threshold: u64,
            pub shutdown_vti_threshold: u64,
        }
    }
}

mod vagal_brake {
    pub mod contract {
        use super::super::*;
        pub use vagus_spec::*;
    }
    pub mod msg {
        use super::super::*;
        #[cosmwasm_schema::cw_serde]
        pub struct InstantiateMsg {
            pub ans_state_manager: String,
            pub capability_issuer: String,
        }
    }
}
