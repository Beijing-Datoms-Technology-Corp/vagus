//! Vagus Gateway Library
//!
//! Device-side gateway that monitors blockchain events, collects telemetry,
//! computes local VTI, and submits afferent evidence packets.

pub mod cbf;
pub mod collector;
pub mod event_watcher;
pub mod manager;
pub mod token_manager;

pub use manager::VagusGateway;
pub use cbf::ControlBarrierFunction;
