//! Vagus Telemetry Library
//!
//! Provides data structures for sensor telemetry and hash commitments
//! used in the afferent evidence processing pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Telemetry data point from a single sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    /// Sensor identifier
    pub sensor_id: String,
    /// Sensor type (e.g., "temperature", "distance", "current")
    pub sensor_type: String,
    /// Reading value
    pub value: f64,
    /// Unit of measurement
    pub unit: String,
    /// Timestamp (Unix timestamp in milliseconds)
    pub timestamp: u64,
}

/// Collection of sensor readings for a time window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryWindow {
    /// Executor ID this telemetry is for
    pub executor_id: u64,
    /// Start time of the window (Unix timestamp in milliseconds)
    pub window_start: u64,
    /// End time of the window (Unix timestamp in milliseconds)
    pub window_end: u64,
    /// Sensor readings in this window
    pub readings: Vec<SensorReading>,
}

/// Aggregated metrics from a telemetry window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowMetrics {
    /// Executor ID
    pub executor_id: u64,
    /// Window start time
    pub window_start: u64,
    /// Window end time
    pub window_end: u64,
    /// Minimum human distance detected (mm)
    pub min_human_distance: Option<f64>,
    /// Maximum temperature (°C)
    pub max_temperature: Option<f64>,
    /// Average energy consumption (J)
    pub avg_energy_consumption: Option<f64>,
    /// Maximum jerk (mm/s²)
    pub max_jerk: Option<f64>,
    /// Battery level remaining (0-100%)
    pub battery_level: Option<f64>,
}

/// Afferent Evidence Packet (AEP) ready for blockchain submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AfferentEvidencePacket {
    /// Executor ID
    pub executor_id: u64,
    /// Current state root hash
    pub state_root: [u8; 32],
    /// Metrics hash for this evidence packet
    pub metrics_hash: [u8; 32],
    /// Attestation signature (if available)
    pub attestation: Option<Vec<u8>>,
    /// Timestamp when evidence was generated
    pub timestamp: u64,
}

/// Vagal Tone Indicator (VTI) computation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VagalToneIndicator {
    /// Computed VTI value (0.0 to 1.0, where 1.0 is most dangerous)
    pub value: f64,
    /// Individual metric contributions
    pub contributions: HashMap<String, f64>,
    /// Computation timestamp
    pub timestamp: u64,
}

/// Pose representation for robotic systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pose {
    /// Position coordinates (x, y, z)
    pub position: [f64; 3],
    /// Orientation as quaternion (w, x, y, z)
    pub orientation: [f64; 4],
}

/// Safety guard result for trajectory planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyGuard {
    /// Whether the action is allowed
    pub allowed: bool,
    /// Scaling factor to apply (0.0 to 1.0)
    pub scaling_factor: f64,
    /// Reason for blocking/ scaling (if applicable)
    pub reason: Option<String>,
}

impl TelemetryWindow {
    /// Create a new telemetry window
    pub fn new(executor_id: u64, window_start: u64, window_end: u64) -> Self {
        Self {
            executor_id,
            window_start,
            window_end,
            readings: Vec::new(),
        }
    }

    /// Add a sensor reading to this window
    pub fn add_reading(&mut self, reading: SensorReading) {
        self.readings.push(reading);
    }

    /// Compute aggregated metrics for this window
    pub fn compute_metrics(&self) -> WindowMetrics {
        let mut min_human_distance = None;
        let mut max_temperature = None;
        let mut energy_readings = Vec::new();
        let mut max_jerk = None;
        let mut battery_level = None;

        for reading in &self.readings {
            match reading.sensor_type.as_str() {
                "human_distance" => {
                    if min_human_distance.is_none() || reading.value < min_human_distance.unwrap() {
                        min_human_distance = Some(reading.value);
                    }
                }
                "temperature" => {
                    if max_temperature.is_none() || reading.value > max_temperature.unwrap() {
                        max_temperature = Some(reading.value);
                    }
                }
                "energy_consumption" => {
                    energy_readings.push(reading.value);
                }
                "jerk" => {
                    if max_jerk.is_none() || reading.value > max_jerk.unwrap() {
                        max_jerk = Some(reading.value);
                    }
                }
                "battery_level" => {
                    battery_level = Some(reading.value);
                }
                _ => {} // Ignore unknown sensor types
            }
        }

        let avg_energy_consumption = if energy_readings.is_empty() {
            None
        } else {
            Some(energy_readings.iter().sum::<f64>() / energy_readings.len() as f64)
        };

        WindowMetrics {
            executor_id: self.executor_id,
            window_start: self.window_start,
            window_end: self.window_end,
            min_human_distance,
            max_temperature,
            avg_energy_consumption,
            max_jerk,
            battery_level,
        }
    }
}

impl WindowMetrics {
    /// Compute hash of the metrics for commitment
    pub fn hash(&self) -> [u8; 32] {
        use sha3::{Digest, Sha3_256};

        let mut hasher = Sha3_256::new();
        hasher.update(self.executor_id.to_be_bytes());
        hasher.update(self.window_start.to_be_bytes());
        hasher.update(self.window_end.to_be_bytes());

        if let Some(dist) = self.min_human_distance {
            hasher.update((dist as u64).to_be_bytes());
        }
        if let Some(temp) = self.max_temperature {
            hasher.update((temp as u64).to_be_bytes());
        }
        if let Some(energy) = self.avg_energy_consumption {
            hasher.update((energy as u64).to_be_bytes());
        }
        if let Some(jerk) = self.max_jerk {
            hasher.update((jerk as u64).to_be_bytes());
        }
        if let Some(battery) = self.battery_level {
            hasher.update((battery as u64).to_be_bytes());
        }

        hasher.finalize().into()
    }
}

impl VagalToneIndicator {
    /// Create a new VTI with zero value
    pub fn new() -> Self {
        Self {
            value: 0.0,
            contributions: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Compute VTI from window metrics using a simple weighted formula
    pub fn from_metrics(metrics: &WindowMetrics) -> Self {
        let mut vti = Self::new();
        let mut total_weight = 0.0;

        // Human distance contribution (lower distance = higher danger)
        if let Some(dist) = metrics.min_human_distance {
            let dist_contrib = if dist < 500.0 {
                1.0 - (dist / 500.0).min(1.0) // Danger when < 500mm
            } else {
                0.0
            };
            vti.contributions.insert("human_distance".to_string(), dist_contrib);
            vti.value += dist_contrib * 0.4; // 40% weight
            total_weight += 0.4;
        }

        // Temperature contribution
        if let Some(temp) = metrics.max_temperature {
            let temp_contrib = if temp > 80.0 {
                ((temp - 80.0) / 20.0).min(1.0) // Danger when > 80°C
            } else {
                0.0
            };
            vti.contributions.insert("temperature".to_string(), temp_contrib);
            vti.value += temp_contrib * 0.2; // 20% weight
            total_weight += 0.2;
        }

        // Energy consumption contribution (higher = more dangerous)
        if let Some(energy) = metrics.avg_energy_consumption {
            let energy_contrib = (energy / 1000.0).min(1.0); // Normalize to 1000J max
            vti.contributions.insert("energy".to_string(), energy_contrib);
            vti.value += energy_contrib * 0.2; // 20% weight
            total_weight += 0.2;
        }

        // Jerk contribution (sudden movements are dangerous)
        if let Some(jerk) = metrics.max_jerk {
            let jerk_contrib = (jerk / 2000.0).min(1.0); // Normalize to 2000 mm/s² max
            vti.contributions.insert("jerk".to_string(), jerk_contrib);
            vti.value += jerk_contrib * 0.2; // 20% weight
            total_weight += 0.2;
        }

        // Normalize by total weight
        if total_weight > 0.0 {
            vti.value /= total_weight;
        }

        vti.value = vti.value.min(1.0); // Clamp to [0, 1]
        vti
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_window_metrics() {
        let mut window = TelemetryWindow::new(42, 1000, 2000);

        window.add_reading(SensorReading {
            sensor_id: "dist_1".to_string(),
            sensor_type: "human_distance".to_string(),
            value: 300.0,
            unit: "mm".to_string(),
            timestamp: 1500,
        });

        window.add_reading(SensorReading {
            sensor_id: "temp_1".to_string(),
            sensor_type: "temperature".to_string(),
            value: 85.0,
            unit: "celsius".to_string(),
            timestamp: 1500,
        });

        let metrics = window.compute_metrics();

        assert_eq!(metrics.min_human_distance, Some(300.0));
        assert_eq!(metrics.max_temperature, Some(85.0));
    }

    #[test]
    fn test_vti_computation() {
        let metrics = WindowMetrics {
            executor_id: 42,
            window_start: 1000,
            window_end: 2000,
            min_human_distance: Some(200.0), // Very close - high danger
            max_temperature: Some(90.0),     // Hot - medium danger
            avg_energy_consumption: Some(500.0), // Medium energy
            max_jerk: Some(1000.0),          // Medium jerk
            battery_level: Some(50.0),
        };

        let vti = VagalToneIndicator::from_metrics(&metrics);

        assert!(vti.value > 0.0 && vti.value <= 1.0);
        assert!(vti.contributions.contains_key("human_distance"));
        assert!(vti.contributions.contains_key("temperature"));
        assert!(vti.contributions.contains_key("energy"));
        assert!(vti.contributions.contains_key("jerk"));
    }
}
