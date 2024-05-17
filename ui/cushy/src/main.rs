#[macro_use]
mod shortcut;
mod settings;
mod widgets;

use cushy::figures::Zero;
use cushy::widgets::Space;
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

use cushy::{Lazy, Run};
use ndoc::{Document, Indentation};
use settings::Settings;
use shortcut::Shortcut;
use widgets::scroll::{MyScroll, ScrollController};

#[derive(Clone, Copy)]
pub struct ViewCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(WidgetId, &TextEditor),
}

#[derive(Clone, Copy)]
pub struct WindowCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(&EditorWindow),
}

pub static VIEW_SHORTCUT: Lazy<Arc<Mutex<HashMap<Shortcut, ViewCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
pub static VIEW_COMMAND_REGISTRY: Lazy<Arc<Mutex<HashMap<&'static str, ViewCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static WINDOW_SHORTCUT: Lazy<Arc<Mutex<HashMap<Shortcut, WindowCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
pub static WINDOW_COMMAND_REGISTRY: Lazy<Arc<Mutex<HashMap<&'static str, WindowCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static FONT_SYSTEM: Lazy<Arc<Mutex<FontSystem>>> =
    Lazy::new(|| Arc::new(Mutex::new(FontSystem::new())));

const NEW_DOC: WindowCommand = WindowCommand {
    name: "New Document",
    id: "window.newdoc",
    action: |w| {
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

pub static SETTINGS: Lazy<Arc<Mutex<Settings>>> =
    Lazy::new(|| Arc::new(Mutex::new(Settings::load())));

pub fn get_settings() -> Settings {
    SETTINGS.lock().unwrap().clone()
}

fn main() -> anyhow::Result<()> {
    let theme = ThemePair::from_scheme(&ColorScheme::from_primary(ColorSource::new(142.0, 0.1)));

    WINDOW_COMMAND_REGISTRY
        .lock()
        .unwrap()
        .insert(NEW_DOC.id, NEW_DOC);
    VIEW_COMMAND_REGISTRY
        .lock()
        .unwrap()
        .insert(GOTO_LINE.id, GOTO_LINE);

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("editor."))
    {
        let mut v = VIEW_SHORTCUT.lock().unwrap();
        let r = VIEW_COMMAND_REGISTRY.lock().unwrap();
        if let Some(cmd) = r.get(command_id.as_str()) {
            dbg!(command_id, shortcut);
            v.insert(shortcut.clone(), *cmd);
        }
    }

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("window."))
    {
        let mut v = WINDOW_SHORTCUT.lock().unwrap();
        let r = WINDOW_COMMAND_REGISTRY.lock().unwrap();
        if let Some(cmd) = r.get(command_id.as_str()) {
            dbg!(command_id, shortcut);
            v.insert(shortcut.clone(), *cmd);
        }
    }

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

    EditorWindow::new(CodeEditor::new(doc.clone()))
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
                .centered().pad_by(Px::new(2)),
        )
        .into_rows()
        .gutter(Px::ZERO)
        .themed(theme)
        .with(&TextSize, Lp::points(10))
        .run()?;

    Ok(())
}
