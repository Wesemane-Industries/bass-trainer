use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::music::{Key, Mode};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StoredConfig {
    pub device_name: Option<String>,
    pub channel: Option<u16>,
    pub key_root_pc: Option<i32>,
    pub key_mode: Option<Mode>,
}

impl StoredConfig {
    pub fn key(&self) -> Option<Key> {
        match (self.key_root_pc, self.key_mode) {
            (Some(pc), Some(mode)) => Some(Key::new(pc, mode)),
            _ => None,
        }
    }

    pub fn set_key(&mut self, key: Key) {
        self.key_root_pc = Some(key.root_pc);
        self.key_mode = Some(key.mode);
    }
}

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "nwesem", "bass-trainer")
        .map(|dirs| dirs.config_dir().join("config.json"))
}

pub fn load() -> StoredConfig {
    let Some(path) = config_path() else {
        return StoredConfig::default();
    };
    let Ok(bytes) = fs::read(&path) else {
        return StoredConfig::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(cfg: &StoredConfig) -> Result<()> {
    let path = config_path().context("could not resolve config directory")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("could not create config dir")?;
    }
    let bytes = serde_json::to_vec_pretty(cfg).context("serializing config")?;
    fs::write(&path, bytes).context("writing config")?;
    Ok(())
}
