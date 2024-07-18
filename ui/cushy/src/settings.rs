use std::collections::HashMap;
use anyhow::Context;
use cushy::kludgine::wgpu::core::resource::CreateBufferError;
use directories::ProjectDirs;
use ndoc::Indentation;
use serde::{Deserialize, Serialize};
use crate::shortcut::Shortcut;
use toml_edit::{de::from_document, DocumentMut};


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    pub shortcuts: HashMap<String, Shortcut>,
    pub indentation: Indentation,
    pub theme: String,
}

#[derive(Deserialize, Serialize, Debug, Default)]
struct OptSettings {
    pub shortcuts: Option<HashMap<String, Shortcut>>,
    pub indentation: Option<Indentation>,
    pub theme: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut shortcuts = HashMap::new();

        shortcuts.insert(crate::GOTO_LINE.id.to_string(),shortcut!(Ctrl + g));
        shortcuts.insert(crate::DUPLICATE_SELECTION_DOWN.id.to_string(),shortcut!(Ctrl + Alt + ArrowDown));
        shortcuts.insert(crate::DUPLICATE_SELECTION_UP.id.to_string(),shortcut!(Ctrl + Alt + ArrowUp));
        shortcuts.insert(crate::DUPLICATE_SELECTION.id.to_string(),shortcut!(Ctrl + d));
        // shortcuts.insert(crate::COPY_SELECTION_CMD.id.to_string(), shortcut!(Ctrl+c));
        // shortcuts.insert(crate::PASTE_SELECTION_CMD.id.to_string(),shortcut!(Ctrl + v));
        // shortcuts.insert(crate::CUT_SELECTION_CMD.id.to_string(),shortcut!(Ctrl + x));
        shortcuts.insert(crate::SAVE_DOC_CMD.id.to_string(),shortcut!(Ctrl + s));
        shortcuts.insert(crate::OPEN_DOC.id.to_string(),shortcut!(Ctrl + o));
        // shortcuts.insert(crate::SAVE_DOC_AS_CMD.id.to_string(),shortcut!(Ctrl + Shift + s));
        shortcuts.insert(crate::UNDO_CMD.id.to_string(),shortcut!(Ctrl + z));
        shortcuts.insert(crate::REDO_CMD.id.to_string(),shortcut!(Ctrl + y));

        shortcuts.insert(crate::NEW_DOC.id.to_string(),shortcut!(Ctrl + n));
        shortcuts.insert(crate::CLOSE_DOC.id.to_string(),shortcut!(Ctrl + w));
        shortcuts.insert(crate::SELECT_DOC.id.to_string(), shortcut!(Ctrl + p));

        shortcuts.insert(crate::NEXT_DOC.id.to_string(),shortcut!(Ctrl+Tab));
        shortcuts.insert(crate::PREV_DOC.id.to_string(),shortcut!(Ctrl+Shift+Tab));
        shortcuts.insert(crate::SHOW_ALL_COMMAND.id.to_string(), shortcut!(Ctrl + Shift + p));
        

        Self { shortcuts, indentation: Default::default(), theme: "base16-eighties.dark".to_string() }
    }
}


impl Settings {
    fn try_load() -> anyhow::Result<Self> {
        dbg!("Loading settings");
        let default_settings = Settings::default();// : Settings = toml::from_str(&config_content).context("Deserializing settings")?;

        
        let config_file = ProjectDirs::from("rs", "", "somepad").context("Getting project config path")?.config_dir().join("settings.toml");

        dbg!("reading settings file from", &config_file);
        let config_content = std::fs::read_to_string(&config_file).context(format!("Reading settings file {}",&config_file.to_string_lossy()))?;
        let toml = config_content.parse::<DocumentMut>().context("Parsing settings")?;
        
        
        
        let settings : OptSettings = from_document(toml)?;
        
        let settings = Settings {
            shortcuts: settings.shortcuts.unwrap_or(default_settings.shortcuts),
            indentation: settings.indentation.unwrap_or(default_settings.indentation),
            theme: settings.theme.unwrap_or(default_settings.theme),
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