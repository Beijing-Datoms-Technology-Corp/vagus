//! Telemetry Collector
//!
//! Collects sensor data from various sources and aggregates it into telemetry windows.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use vagus_telemetry::{SensorReading, TelemetryWindow, WindowMetrics, VagalToneIndicator};

/// Telemetry collector for aggregating sensor data
#[derive(Clone)]
pub struct TelemetryCollector {
    /// Current telemetry windows per executor
    windows: Arc<RwLock<HashMap<u64, TelemetryWindow>>>,
    /// Window duration in milliseconds
    window_duration_ms: u64,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new(window_duration_ms: u64) -> Self {
        Self {
            windows: Arc::new(RwLock::new(HashMap::new())),
            window_duration_ms,
        }
    }

    /// Add a sensor reading to the appropriate window
    pub async fn add_reading(&self, executor_id: u64, reading: SensorReading) -> Result<()> {
        let mut windows = self.windows.write().await;

        let window = windows.entry(executor_id).or_insert_with(|| {
            let window_start = reading.timestamp / self.window_duration_ms * self.window_duration_ms;
            let window_end = window_start + self.window_duration_ms;

            TelemetryWindow::new(executor_id, window_start, window_end)
        });

        // Check if we need to start a new window
        if reading.timestamp >= window.window_end {
            let new_window_start = reading.timestamp / self.window_duration_ms * self.window_duration_ms;
            let new_window_end = new_window_start + self.window_duration_ms;

            *window = TelemetryWindow::new(executor_id, new_window_start, new_window_end);
        }

        window.add_reading(reading);
        Ok(())
    }

    /// Get current window metrics for an executor
    pub async fn get_current_metrics(&self, executor_id: u64) -> Result<Option<WindowMetrics>> {
        let windows = self.windows.read().await;
        Ok(windows.get(&executor_id).map(|window| window.compute_metrics()))
    }

    /// Get current window for an executor
    pub async fn get_current_window(&self, executor_id: u64) -> Result<Option<TelemetryWindow>> {
        let windows = self.windows.read().await;
        Ok(windows.get(&executor_id).cloned())
    }

    /// Compute VTI for current window
    pub async fn compute_vti(&self, executor_id: u64) -> Result<Option<VagalToneIndicator>> {
        if let Some(metrics) = self.get_current_metrics(executor_id).await? {
            Ok(Some(VagalToneIndicator::from_metrics(&metrics)))
        } else {
            Ok(None)
        }
    }

    /// Clean up old windows
    pub async fn cleanup_old_windows(&self, current_time: u64, max_age_ms: u64) -> Result<()> {
        let mut windows = self.windows.write().await;
        let cutoff_time = current_time.saturating_sub(max_age_ms);

        windows.retain(|_executor_id, window| window.window_end > cutoff_time);
        Ok(())
    }

    /// Get all active executor IDs
    pub async fn get_active_executors(&self) -> Result<Vec<u64>> {
        let windows = self.windows.read().await;
        Ok(windows.keys().cloned().collect())
    }
}

/// Mock sensor data generator for testing
pub struct MockSensorDataGenerator {
    executor_id: u64,
    base_timestamp: u64,
}

impl MockSensorDataGenerator {
    pub fn new(executor_id: u64) -> Self {
        Self {
            executor_id,
            base_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Generate a batch of mock sensor readings
    pub fn generate_readings(&mut self, count: usize) -> Vec<SensorReading> {
        let mut readings = Vec::new();

        for i in 0..count {
            let timestamp = self.base_timestamp + (i as u64 * 100); // 100ms intervals

            // Generate different types of readings
            let reading_type = i % 4;
            let reading = match reading_type {
                0 => SensorReading {
                    sensor_id: format!("dist_{}", i),
                    sensor_type: "human_distance".to_string(),
                    value: 300.0 + (i as f64 * 10.0), // Increasing distance
                    unit: "mm".to_string(),
                    timestamp,
                },
                1 => SensorReading {
                    sensor_id: format!("temp_{}", i),
                    sensor_type: "temperature".to_string(),
                    value: 50.0 + (i as f64 * 2.0), // Increasing temperature
                    unit: "celsius".to_string(),
                    timestamp,
                },
                2 => SensorReading {
                    sensor_id: format!("energy_{}", i),
                    sensor_type: "energy_consumption".to_string(),
                    value: 100.0 + (i as f64 * 5.0), // Increasing energy
                    unit: "joules".to_string(),
                    timestamp,
                },
                3 => SensorReading {
                    sensor_id: format!("jerk_{}", i),
                    sensor_type: "jerk".to_string(),
                    value: 1.0 + (i as f64 * 0.1), // Small jerk values
                    unit: "m/s²".to_string(),
                    timestamp,
                },
                _ => unreachable!(),
            };

            readings.push(reading);
        }

        self.base_timestamp += count as u64 * 100;
        readings
    }

    /// Generate dangerous readings (for testing reflex triggers)
    pub fn generate_dangerous_readings(&mut self) -> Vec<SensorReading> {
        let timestamp = self.base_timestamp;

        vec![
            SensorReading {
                sensor_id: "dist_danger".to_string(),
                sensor_type: "human_distance".to_string(),
                value: 150.0, // Very close human
                unit: "mm".to_string(),
                timestamp,
            },
            SensorReading {
                sensor_id: "temp_danger".to_string(),
                sensor_type: "temperature".to_string(),
                value: 95.0, // Very hot
                unit: "celsius".to_string(),
                timestamp,
            },
            SensorReading {
                sensor_id: "energy_danger".to_string(),
                sensor_type: "energy_consumption".to_string(),
                value: 1500.0, // High energy consumption
                unit: "joules".to_string(),
                timestamp,
            },
            SensorReading {
                sensor_id: "jerk_danger".to_string(),
                sensor_type: "jerk".to_string(),
                value: 1500.0, // High jerk
                unit: "m/s²".to_string(),
                timestamp,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_collection() {
        let collector = TelemetryCollector::new(1000); // 1 second windows

        let reading = SensorReading {
            sensor_id: "test_sensor".to_string(),
            sensor_type: "human_distance".to_string(),
            value: 500.0,
            unit: "mm".to_string(),
            timestamp: 1000,
        };

        // Add reading
        collector.add_reading(42, reading).await.unwrap();

        // Check metrics
        let metrics = collector.get_current_metrics(42).await.unwrap().unwrap();
        assert_eq!(metrics.min_human_distance, Some(500.0));

        // Check VTI computation
        let vti = collector.compute_vti(42).await.unwrap().unwrap();
        assert!(vti.value >= 0.0 && vti.value <= 1.0);
    }

    #[tokio::test]
    async fn test_window_rollover() {
        let collector = TelemetryCollector::new(1000);

        // Add reading to first window
        let reading1 = SensorReading {
            sensor_id: "test1".to_string(),
            sensor_type: "human_distance".to_string(),
            value: 500.0,
            unit: "mm".to_string(),
            timestamp: 500, // In first window (0-1000)
        };

        collector.add_reading(42, reading1).await.unwrap();

        // Add reading that should create new window
        let reading2 = SensorReading {
            sensor_id: "test2".to_string(),
            sensor_type: "human_distance".to_string(),
            value: 300.0,
            unit: "mm".to_string(),
            timestamp: 1500, // In second window (1000-2000)
        };

        collector.add_reading(42, reading2).await.unwrap();

        // Current metrics should reflect the new window
        let metrics = collector.get_current_metrics(42).await.unwrap().unwrap();
        assert_eq!(metrics.min_human_distance, Some(300.0));
    }

    #[test]
    fn test_mock_sensor_generator() {
        let mut generator = MockSensorDataGenerator::new(42);

        let readings = generator.generate_readings(8);
        assert_eq!(readings.len(), 8);

        // Check that we have different sensor types
        let types: std::collections::HashSet<_> = readings.iter()
            .map(|r| r.sensor_type.clone())
            .collect();
        assert_eq!(types.len(), 4); // Should have 4 different types
    }
}
