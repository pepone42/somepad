#[macro_use]
mod shortcut;
mod settings;
mod widgets;

use cushy::debug::DebugContext;
use cushy::figures::Zero;
use cushy::widgets::{Custom, Space};
use widgets::editor_window::EditorWindow;
use widgets::palette::ask;
use widgets::text_editor::{self, CodeEditor, TextEditor};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cushy::figures::units::{Lp, Px};

use cushy::kludgine::cosmic_text::FontSystem;
use cushy::styles::components::{CornerRadius, TextSize};
use cushy::styles::{Color, ColorScheme, ColorSource, CornerRadii, Dimension, ThemePair};
use cushy::value::{Dynamic, Source};
use cushy::widget::{MakeWidget, WidgetId};

use cushy::{Lazy, Open, PendingApp, Run};
use ndoc::{Document, Indentation};
use settings::Settings;
use shortcut::Shortcut;
use widgets::scroll::{MyScroll, ScrollController};

#[derive(Debug, Clone, Copy)]
pub struct ViewCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(WidgetId, &TextEditor),
}

#[derive(Debug, Clone, Copy)]
pub struct WindowCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(WidgetId, &EditorWindow),
}

pub static FONT_SYSTEM: Lazy<Arc<Mutex<FontSystem>>> =
    Lazy::new(|| Arc::new(Mutex::new(FontSystem::new())));

const NEW_DOC: WindowCommand = WindowCommand {
    name: "New Document",
    id: "window.newdoc",
    action: |_id, w| {
        dbg!("New doc!");
        w.add_new_doc(Dynamic::new(Document::default()));
    },
};

const GOTO_LINE: ViewCommand = ViewCommand {
    name: "Go to Line",
    id: "editor.goto_line",
    action: |id, v| {
        let doc = v.doc.clone();

        ask(id, "Got to line", move |c, _, s| {
            if let Ok(line) = s.parse::<usize>() {
                if line == 0 || line > doc.get().rope.len_lines() {
                    return;
                }

                let p = ndoc::Position::new(line - 1, 0);
                doc.lock().set_main_selection(p, p);

                c.widget()
                    .lock()
                    .downcast_ref::<TextEditor>()
                    .unwrap()
                    .refocus_main_selection();
            }
        });
    },
};

const UNDO_CMD: ViewCommand = ViewCommand {
    name: "Undo",
    id: "editor.undo",
    action: |_id, v| {
        v.doc.lock().undo();
        v.refocus_main_selection();
    },
};
const REDO_CMD: ViewCommand = ViewCommand {
    name: "redo",
    id: "editor.redo",
    action: |_id, v| {
        v.doc.lock().redo();
        v.refocus_main_selection();
    },
};

pub static SETTINGS: Lazy<Arc<Mutex<Settings>>> =
    Lazy::new(|| Arc::new(Mutex::new(Settings::load())));

pub fn get_settings() -> Settings {
    SETTINGS.lock().unwrap().clone()
}

#[derive(Debug, Clone)]
pub struct CommandsRegistry {
    pub view: HashMap<&'static str, ViewCommand>,
    pub window: HashMap<&'static str, WindowCommand>,
    pub view_shortcut: HashMap<Shortcut, ViewCommand>,
    pub window_shortcut: HashMap<Shortcut, WindowCommand>,
}

impl CommandsRegistry {
    pub fn new() -> Self {
        Self {
            view: HashMap::new(),
            window: HashMap::new(),
            view_shortcut: HashMap::new(),
            window_shortcut: HashMap::new(),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let theme = ThemePair::from_scheme(&ColorScheme::from_primary(ColorSource::new(142.0, 0.1)));

    let mut cmd_reg = CommandsRegistry::new();

    cmd_reg.window.insert(NEW_DOC.id, NEW_DOC);
    cmd_reg.view.insert(GOTO_LINE.id, GOTO_LINE);
    cmd_reg.view.insert(UNDO_CMD.id, UNDO_CMD);
    cmd_reg.view.insert(REDO_CMD.id, REDO_CMD);

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("editor."))
    {
        if let Some(cmd) = cmd_reg.view.get(command_id.as_str()) {
            dbg!(command_id, shortcut);
            cmd_reg.view_shortcut.insert(shortcut.clone(), *cmd);
        }
    }

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("window."))
    {
        if let Some(cmd) = cmd_reg.window.get(command_id.as_str()) {
            dbg!(command_id, shortcut);
            cmd_reg.window_shortcut.insert(shortcut.clone(), *cmd);
        }
    }

    let cmd_reg = Dynamic::new(cmd_reg);

    ndoc::Document::init_highlighter();
    let doc = Dynamic::new(if let Some(path) = std::env::args().nth(1) {
        ndoc::Document::from_file(path)?
    } else {
        ndoc::Document::default()
    });
    let file_name = doc.map_each(move |d| {
        format!(
            "{}{}",
            if let Some(file_name) = &d.file_name {
                file_name.file_name().unwrap().to_string_lossy().to_string()
            } else {
                "Untilted".to_string()
            },
            if d.is_dirty() { "*" } else { "" }
        )
    });
    let selection = doc.map_each(|d| {
        if d.selections.len() > 1 {
            format!("{} selections", d.selections.len())
        } else {
            format!(
                "Ln {}, Col {}",
                d.selections[0].head.line + 1,
                d.selections[0].head.column + 1
            )
        }
    });
    let indentation = doc.map_each(|d| match d.file_info.indentation {
        Indentation::Space(spaces) => format!("Space({})", spaces),
        Indentation::Tab(spaces) => format!("Tab({})", spaces),
    });
    let encoding = doc.map_each(|d| d.file_info.encoding.name());
    let eol = doc.map_each(|d| match d.file_info.linefeed {
        ndoc::LineFeed::CR => "CR",
        ndoc::LineFeed::LF => "LF",
        ndoc::LineFeed::CRLF => "CRLF",
    });
    let syntax = doc.map_each(|d| d.file_info.syntax.name.clone());

    EditorWindow::new(CodeEditor::new(doc.clone(), cmd_reg.clone()), cmd_reg.clone())
        .expand()
        .and(
            file_name
                .align_left()
                .and(Space::clear().expand_horizontally())
                .and(selection)
                .and(indentation)
                .and(encoding)
                .and(eol)
                .and(syntax)
                .into_columns()
                .centered()
                .pad_by(Px::new(2)),
        )
        .into_rows()
        .gutter(Px::ZERO)
        .themed(theme)
        .with(&TextSize, Lp::points(10))
        .run()?;

    Ok(())
}
