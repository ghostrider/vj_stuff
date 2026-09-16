use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const MIN_OFFSET_MS: i32 = -4000;
pub const MAX_OFFSET_MS: i32 = 4000;
pub const DEFAULT_OFFSET_MS: i32 = 200;

pub const MIN_TRIM_MS: i32 = -50;
pub const MAX_TRIM_MS: i32 = 50;

pub const RESYNC_QUANTA_BARS: [u8; 3] = [1, 2, 4];
pub const DEFAULT_RESYNC_QUANTUM_BARS: u8 = 1;

pub const DEFAULT_VIRTUAL_IN_NAME: &str = "Clock Shifter In";
pub const DEFAULT_VIRTUAL_OUT_NAME: &str = "Clock Shifter Out";

pub fn clamp_offset_ms(ms: i32) -> i32 {
    ms.clamp(MIN_OFFSET_MS, MAX_OFFSET_MS)
}

pub fn clamp_trim_ms(ms: i32) -> i32 {
    ms.clamp(MIN_TRIM_MS, MAX_TRIM_MS)
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", content = "name")]
pub enum InputSource {
    #[default]
    VirtualIn,
    Hardware(String),
    Simulator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputSettings {
    pub enabled: bool,
    pub trim_ms: i32,
    pub forward_transport: bool,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            trim_ms: 0,
            forward_transport: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulatorSettings {
    pub bpm: f64,
    pub jitter_ms: f64,
}

impl Default for SimulatorSettings {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            jitter_ms: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub global_offset_ms: i32,
    pub input_source: InputSource,
    pub resync_quantum_bars: u8,
    pub virtual_in_name: String,
    pub virtual_out_name: String,
    pub outputs: BTreeMap<String, OutputSettings>,
    pub forward_continue: bool,
    pub simulator: SimulatorSettings,
    /// Measure panel (PROMPTS.md Prompt 7): a second, independently-selected
    /// MIDI input port name to validate real-world lead against. `None`
    /// disables the feature.
    pub measure_input: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            global_offset_ms: DEFAULT_OFFSET_MS,
            input_source: InputSource::default(),
            resync_quantum_bars: DEFAULT_RESYNC_QUANTUM_BARS,
            virtual_in_name: DEFAULT_VIRTUAL_IN_NAME.to_string(),
            virtual_out_name: DEFAULT_VIRTUAL_OUT_NAME.to_string(),
            outputs: BTreeMap::new(),
            forward_continue: false,
            simulator: SimulatorSettings::default(),
            measure_input: None,
        }
    }
}

impl AppConfig {
    /// Default config file location: `%APPDATA%\midi_clock_shifter\config.json`.
    pub fn default_path() -> Option<PathBuf> {
        directories::BaseDirs::new().map(|dirs| {
            dirs.config_dir()
                .join("midi_clock_shifter")
                .join("config.json")
        })
    }

    /// Loads config from `path` (or the default location if `None`), falling back to
    /// defaults if the file is missing, unreadable, or malformed.
    pub fn load(path: Option<&Path>) -> Self {
        let resolved = match path.map(Path::to_path_buf).or_else(Self::default_path) {
            Some(p) => p,
            None => {
                log::warn!("could not determine config directory, using defaults");
                return Self::default();
            }
        };

        match fs::read_to_string(&resolved) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_else(|err| {
                log::warn!("failed to parse config at {}: {err}", resolved.display());
                Self::default()
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(err) => {
                log::warn!("failed to read config at {}: {err}", resolved.display());
                Self::default()
            }
        }
    }

    /// Saves config as pretty JSON to `path` (or the default location if `None`).
    pub fn save(&self, path: Option<&Path>) -> Result<()> {
        let resolved = match path.map(Path::to_path_buf).or_else(Self::default_path) {
            Some(p) => p,
            None => anyhow::bail!("could not determine config directory"),
        };

        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating config directory {}", parent.display()))?;
        }

        let data = serde_json::to_string_pretty(self).context("serializing config")?;
        fs::write(&resolved, data)
            .with_context(|| format!("writing config to {}", resolved.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_offset_range() {
        assert_eq!(clamp_offset_ms(-9000), MIN_OFFSET_MS);
        assert_eq!(clamp_offset_ms(9000), MAX_OFFSET_MS);
        assert_eq!(clamp_offset_ms(200), 200);
    }

    #[test]
    fn clamps_trim_range() {
        assert_eq!(clamp_trim_ms(-100), MIN_TRIM_MS);
        assert_eq!(clamp_trim_ms(100), MAX_TRIM_MS);
        assert_eq!(clamp_trim_ms(10), 10);
    }

    #[test]
    fn round_trips_through_json() {
        let mut outputs = BTreeMap::new();
        outputs.insert(
            "Beatstep Pro".to_string(),
            OutputSettings {
                enabled: true,
                trim_ms: -10,
                forward_transport: false,
            },
        );
        let config = AppConfig {
            global_offset_ms: 300,
            input_source: InputSource::Hardware("Beatstep Pro".to_string()),
            outputs,
            ..AppConfig::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, restored);
    }

    #[test]
    fn falls_back_to_defaults_on_malformed_json() {
        let dir =
            std::env::temp_dir().join(format!("midi_clock_shifter_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        fs::write(&path, "not valid json").unwrap();

        let loaded = AppConfig::load(Some(&path));
        assert_eq!(loaded, AppConfig::default());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "midi_clock_shifter_test_save_{}",
            std::process::id()
        ));
        let path = dir.join("config.json");

        let config = AppConfig {
            global_offset_ms: -1500,
            ..AppConfig::default()
        };
        config.save(Some(&path)).unwrap();

        let loaded = AppConfig::load(Some(&path));
        assert_eq!(loaded, config);

        fs::remove_dir_all(&dir).ok();
    }
}
