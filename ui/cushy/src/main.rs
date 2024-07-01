#[macro_use]
mod shortcut;
mod settings;
mod widgets;

use cushy::context::EventContext;
use cushy::figures::Zero;
use cushy::kludgine::app::winit::platform::windows::WindowExtWindows;
use rfd::FileDialog;
use widgets::editor_window::EditorWindow;
use widgets::palette::{Palette, PaletteExt};
use widgets::status_bar::StatusBar;
use widgets::text_editor::TextEditor;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cushy::figures::units::{Lp, Px};

use cushy::kludgine::cosmic_text::FontSystem;
use cushy::styles::components::TextSize;
use cushy::styles::{ColorScheme, ColorSource, ThemePair};
use cushy::value::{Dynamic, Source};
use cushy::widget::{MakeWidget, WidgetId};

use cushy::{Lazy, Run};
use ndoc::Document;
use settings::Settings;
use shortcut::Shortcut;

#[derive(Debug, Clone, Copy)]
pub struct ViewCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(WidgetId, &TextEditor, &mut EventContext),
}

#[derive(Debug, Clone, Copy)]
pub struct WindowCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(WidgetId, &EditorWindow, &mut EventContext),
}

pub static FONT_SYSTEM: Lazy<Arc<Mutex<FontSystem>>> =
    Lazy::new(|| Arc::new(Mutex::new(FontSystem::new())));

const NEW_DOC: WindowCommand = WindowCommand {
    name: "New Document",
    id: "window.newdoc",
    action: |_id, w, c| {
        dbg!("New doc!");
        w.add_new_doc(Dynamic::new(Document::default()), c);
    },
};

const GOTO_LINE: ViewCommand = ViewCommand {
    name: "Go to Line",
    id: "editor.goto_line",
    action: |_id, v, c| {
        let doc = v.doc.clone();

        c.ask("Got to line", move |c, _, s| {
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
    action: |_id, v, _c| {
        v.doc.lock().undo();
        v.refocus_main_selection();
    },
};
const REDO_CMD: ViewCommand = ViewCommand {
    name: "redo",
    id: "editor.redo",
    action: |_id, v, _c| {
        v.doc.lock().redo();
        v.refocus_main_selection();
    },
};

// const COPY_SELECTION_CMD: ViewCommand = ViewCommand {
//     name: "Copy Selection",
//     id: "editor.copyselection",
//     action: |_id, v, c| {
//         if let Some(mut clipboard) = c.cushy().clipboard_guard() {
//             let _ = clipboard.set_text(dbg!(v.doc.get().get_selection_content()));
//         }
//     },
// };

// const CUT_SELECTION_CMD: ViewCommand = ViewCommand {
//     name: "Cut Selection",
//     id: "editor.cutselection",
//     action: |_id, v, c| {
//         if let Some(mut clipboard) = c.cushy().clipboard_guard() {
//             if v.doc.get().get_selection_content().len() > 0 {
//                 let _ = clipboard.set_text(dbg!(v.doc.get().get_selection_content()));
//                 v.doc.lock().insert("");
//                 v.refocus_main_selection();
//             }
//         }
//     },
// };

// const PASTE_SELECTION_CMD: ViewCommand = ViewCommand {
//     name: "Paste Selection",
//     id: "editor.pasteselection",
//     action: |_id, v, c| {
//         if let Some(mut clipboard) = c.cushy().clipboard_guard() {
//             if let Ok(s) = clipboard.get_text() {
//                 v.doc.lock().insert_many(&s);
//                 v.refocus_main_selection();
//             }
//         }
//     },
// };

const SAVE_DOC_CMD: ViewCommand = ViewCommand {
    name: "Save document",
    id: "editor.save_doc",
    action: |_id, v, c| {
        if let Some(ref file_name) = v.doc.get().file_name {
            v.doc.lock().save_as(file_name).unwrap();
        } else {
            v.save_as(c);
        }
    },
};

const OPEN_DOC: WindowCommand = WindowCommand {
    name: "Open Document",
    id: "window.opendoc",
    action: |_id, w, context| {
        context.window_mut().winit().unwrap().set_enable(false);
        if let Some(file) = FileDialog::new().pick_file() {
            // TODO: check for errors
            let doc = Document::from_file(file).unwrap();
            w.add_new_doc(Dynamic::new(doc), context)
        }
        context.window_mut().winit().unwrap().set_enable(true);
        context.window_mut().winit().unwrap().focus_window();
    },
};

const CLOSE_DOC: WindowCommand = WindowCommand {
    name: "Close Document",
    id: "window.closedoc",
    action: |_id, w, _c| {
        let current_doc = w.current_doc.get();
        let docs_len = w.documents.get().len();
        if docs_len > 1 {
            w.documents.lock().remove(current_doc);
            *w.current_doc.lock() -= 1;
        }
        // TODO: close the window if there is only one doc
        // TODO: warn if the doc is dirty
    },
};

const NEXT_DOC: WindowCommand = WindowCommand {
    name: "Next Document",
    id: "window.nextdoc",
    action: |_id, w, _c| {
        let current_doc = w.current_doc.get();
        let docs_len = w.documents.get().len();
        if current_doc + 1 < docs_len {
            *w.current_doc.lock() += 1;
        } else {
            *w.current_doc.lock() = 0;
        }
    },
};

const SELECT_DOC: WindowCommand = WindowCommand {
    name: "Select Document",
    id: "window.select_doc",
    action: |_id, w, c| {
        let items = w
            .documents
            .get()
            .iter()
            .map(|d| {
                if let Some(file_name) = d.get().file_name {
                    file_name.file_name().unwrap().to_string_lossy().to_string()
                } else {
                    "Untitled".to_string()
                }
            })
            .collect();
        c.choose( "Select a document", items, |_,i,val| {
            dbg!("Selected!",i,val);
        })
    }
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
    cmd_reg.window.insert(NEXT_DOC.id, NEXT_DOC);

    cmd_reg.view.insert(GOTO_LINE.id, GOTO_LINE);
    cmd_reg.view.insert(UNDO_CMD.id, UNDO_CMD);
    cmd_reg.view.insert(REDO_CMD.id, REDO_CMD);
    // cmd_reg
    //     .view
    //     .insert(COPY_SELECTION_CMD.id, COPY_SELECTION_CMD);
    // cmd_reg
    //     .view
    //     .insert(PASTE_SELECTION_CMD.id, PASTE_SELECTION_CMD);
    // cmd_reg.view.insert(CUT_SELECTION_CMD.id, CUT_SELECTION_CMD);
    cmd_reg.view.insert(SAVE_DOC_CMD.id, SAVE_DOC_CMD);
    cmd_reg.window.insert(OPEN_DOC.id, OPEN_DOC);
    cmd_reg.window.insert(CLOSE_DOC.id, CLOSE_DOC);
    cmd_reg.window.insert(SELECT_DOC.id, SELECT_DOC);

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

    let editor = EditorWindow::new(doc.clone(), cmd_reg.clone());

    let docs = editor.documents.clone();
    let cur_doc = editor.current_doc.clone();

    editor
        .expand()
        .and(StatusBar::new(docs, cur_doc).centered().pad_by(Px::new(2)))
        .into_rows()
        .gutter(Px::ZERO)
        .themed(theme)
        .with(&TextSize, Lp::points(10))
        .run()?;

    Ok(())
}
