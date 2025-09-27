//! Control Barrier Function (CBF) Interface
//!
//! Provides safety constraints for trajectory planning and execution.

use async_trait::async_trait;
use vagus_telemetry::{Pose, SafetyGuard};

/// Control Barrier Function interface for safety constraints
#[async_trait]
pub trait ControlBarrierFunction: Send + Sync {
    /// Check if a pose setpoint is safe given current sensor readings
    /// Returns a SafetyGuard indicating if the pose is allowed and any scaling needed
    async fn guard(&self, setpoint: &Pose, sensor_data: &SensorData) -> anyhow::Result<SafetyGuard>;

    /// Update CBF parameters based on current conditions
    async fn update_parameters(&mut self, conditions: &SafetyConditions) -> anyhow::Result<()>;
}

/// Sensor data input for CBF
#[derive(Debug, Clone)]
pub struct SensorData {
    pub human_distances: Vec<f64>, // Distances to humans in mm
    pub temperatures: Vec<f64>,    // Temperatures in °C
    pub velocities: Vec<f64>,      // Current velocities in m/s
    pub jerks: Vec<f64>,          // Current jerks in m/s²
    pub battery_level: Option<f64>, // Battery level 0-100%
}

/// Safety condition parameters for CBF
#[derive(Debug, Clone)]
pub struct SafetyConditions {
    pub ans_state: String,        // Current ANS state ("SAFE", "DANGER", "SHUTDOWN")
    pub scaling_factor: f64,      // Current scaling factor from ANS
    pub vti_value: f64,          // Current VTI value
}

/// Basic CBF implementation (placeholder)
pub struct BasicCBF {
    max_human_distance: f64,
    max_temperature: f64,
    max_velocity: f64,
    max_jerk: f64,
}

impl BasicCBF {
    pub fn new() -> Self {
        Self {
            max_human_distance: 300.0, // 300mm minimum distance
            max_temperature: 80.0,     // 80°C max temperature
            max_velocity: 2.0,         // 2 m/s max velocity
            max_jerk: 5.0,            // 5 m/s² max jerk
        }
    }

    pub fn with_limits(
        max_human_distance: f64,
        max_temperature: f64,
        max_velocity: f64,
        max_jerk: f64,
    ) -> Self {
        Self {
            max_human_distance,
            max_temperature,
            max_velocity,
            max_jerk,
        }
    }
}

#[async_trait]
impl ControlBarrierFunction for BasicCBF {
    async fn guard(&self, setpoint: &Pose, sensor_data: &SensorData) -> anyhow::Result<SafetyGuard> {
        // Check human safety
        let min_human_dist = sensor_data.human_distances.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        if min_human_dist < self.max_human_distance {
            return Ok(SafetyGuard {
                allowed: false,
                scaling_factor: 0.0,
                reason: Some("Human too close".to_string()),
            });
        }

        // Check temperature safety
        let max_temp = sensor_data.temperatures.iter().fold(0.0f64, |a, &b| a.max(b));
        if max_temp > self.max_temperature {
            return Ok(SafetyGuard {
                allowed: false,
                scaling_factor: 0.0,
                reason: Some("Temperature too high".to_string()),
            });
        }

        // Check velocity limits
        let max_vel = sensor_data.velocities.iter().fold(0.0f64, |a, &b| a.max(b));
        if max_vel > self.max_velocity {
            return Ok(SafetyGuard {
                allowed: false,
                scaling_factor: 0.0,
                reason: Some("Velocity too high".to_string()),
            });
        }

        // Check jerk limits
        let max_jerk = sensor_data.jerks.iter().fold(0.0f64, |a, &b| a.max(b));
        if max_jerk > self.max_jerk {
            return Ok(SafetyGuard {
                allowed: false,
                scaling_factor: 0.0,
                reason: Some("Jerk too high".to_string()),
            });
        }

        // All checks passed
        Ok(SafetyGuard {
            allowed: true,
            scaling_factor: 1.0,
            reason: None,
        })
    }

    async fn update_parameters(&mut self, conditions: &SafetyConditions) -> anyhow::Result<()> {
        // Adjust limits based on ANS state
        match conditions.ans_state.as_str() {
            "SAFE" => {
                // Normal limits
                self.max_velocity = 2.0;
                self.max_jerk = 5.0;
            }
            "DANGER" => {
                // Reduced limits
                self.max_velocity = 2.0 * conditions.scaling_factor;
                self.max_jerk = 5.0 * conditions.scaling_factor;
            }
            "SHUTDOWN" => {
                // Emergency stop
                self.max_velocity = 0.0;
                self.max_jerk = 0.0;
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_cbf_safe_conditions() {
        let cbf = BasicCBF::new();
        let setpoint = Pose {
            position: [1.0, 2.0, 3.0],
            orientation: [0.0, 0.0, 0.0, 1.0],
        };

        let sensor_data = SensorData {
            human_distances: vec![500.0, 600.0], // Safe distances
            temperatures: vec![50.0, 60.0],      // Safe temperatures
            velocities: vec![1.0, 1.5],          // Safe velocities
            jerks: vec![2.0, 3.0],              // Safe jerks
            battery_level: Some(80.0),
        };

        let guard = cbf.guard(&setpoint, &sensor_data).await.unwrap();
        assert!(guard.allowed);
        assert_eq!(guard.scaling_factor, 1.0);
        assert!(guard.reason.is_none());
    }

    #[tokio::test]
    async fn test_basic_cbf_human_too_close() {
        let cbf = BasicCBF::new();
        let setpoint = Pose {
            position: [1.0, 2.0, 3.0],
            orientation: [0.0, 0.0, 0.0, 1.0],
        };

        let sensor_data = SensorData {
            human_distances: vec![200.0, 600.0], // One human too close
            temperatures: vec![50.0, 60.0],
            velocities: vec![1.0, 1.5],
            jerks: vec![2.0, 3.0],
            battery_level: Some(80.0),
        };

        let guard = cbf.guard(&setpoint, &sensor_data).await.unwrap();
        assert!(!guard.allowed);
        assert_eq!(guard.scaling_factor, 0.0);
        assert_eq!(guard.reason, Some("Human too close".to_string()));
    }
}
