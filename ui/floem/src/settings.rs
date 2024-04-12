use std::{collections::HashMap, default, fs::File, io::Read, vec};
use anyhow::Context;
use directories::ProjectDirs;
use ndoc::Indentation;
use serde::{Deserialize, Serialize};
use crate::{settings, shortcut::Shortcut};
use toml_edit::{de::from_document, value, DocumentMut};


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    pub shortcuts: HashMap<String, Shortcut>,
    pub indentation: Indentation,
}

#[derive(Deserialize, Serialize, Debug, Default)]
struct OptSettings {
    pub shortcuts: Option<HashMap<String, Shortcut>>,
    pub indentation: Option<Indentation>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut shortcuts = HashMap::new();

        shortcuts.insert("gotoline".to_string(),shortcut!(Ctrl + g));
        shortcuts.insert("copyselection".to_string(), shortcut!(Ctrl+c));
        shortcuts.insert("pasteselection".to_string(),shortcut!(Ctrl + v));
        shortcuts.insert("cutselection".to_string(),shortcut!(Ctrl + x));
        shortcuts.insert("savedoc".to_string(),shortcut!(Ctrl + s));
        shortcuts.insert("savedocas".to_string(),shortcut!(Ctrl + Shift + s));
        shortcuts.insert("undo".to_string(),shortcut!(Ctrl + z));
        shortcuts.insert("redo".to_string(),shortcut!(Ctrl + y));


        Self { shortcuts, indentation: Default::default() }
    }
}


impl Settings {
    fn try_load() -> anyhow::Result<Self> {
        
        let default_settings = Settings::default();// : Settings = toml::from_str(&config_content).context("Deserializing settings")?;

        
        let config_file = ProjectDirs::from("rs", "", "somepad").context("Getting project config path")?.config_dir().join("settings.toml");

        
        let config_content = std::fs::read_to_string(&config_file).context(format!("Reading settings file {}",&config_file.to_string_lossy()))?;
        let toml = config_content.parse::<DocumentMut>().context("Parsing settings")?;
        
        
        
        let settings : OptSettings = from_document(toml)?;
        
        let settings = Settings {
            shortcuts: settings.shortcuts.unwrap_or(default_settings.shortcuts),
            indentation: settings.indentation.unwrap_or(default_settings.indentation),
        };
        
        Ok(dbg!(settings))
    }

    pub fn load() -> Self {
        if let Ok(settings) = Settings::try_load().context("Loading settings") {
            settings
        } else {
            Settings::default()
        }
    }
}