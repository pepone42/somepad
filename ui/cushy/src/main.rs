#[macro_use]
mod shortcut;
mod settings;
mod widgets;

use cushy::context::EventContext;
use cushy::figures::Zero;
use cushy::kludgine::app::winit::dpi::{LogicalSize, Size};
use cushy::kludgine::app::winit::platform::windows::WindowExtWindows;
use ndoc::syntax::ThemeSetRegistry;
use rfd::FileDialog;
use widgets::editor_switcher::EditorSwitcher;
use widgets::editor_window::EditorWindow;
use widgets::palette::{palette, PaletteExt};
use widgets::status_bar::StatusBar;
use widgets::text_editor::{CodeEditor, TextEditor};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cushy::figures::units::{Lp, Px};

use cushy::kludgine::cosmic_text::FontSystem;
use cushy::styles::components::{self};
use cushy::styles::{
    ColorSchemeBuilder, ColorSource, CornerRadii, Dimension, ThemePair,
};
use cushy::value::{Dynamic, Source, Value};
use cushy::widget::{MakeWidget, MakeWidgetWithTag, WidgetId, WidgetTag};

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
        c.palette("Got to line")
            .accept(move |c, _, s| {
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
                        .refocus_main_selection(c);
                }
            })
            .show();
    },
};

const UNDO_CMD: ViewCommand = ViewCommand {
    name: "Undo",
    id: "editor.undo",
    action: |_id, v, c| {
        v.doc.lock().undo();
        v.refocus_main_selection(c);
    },
};
const REDO_CMD: ViewCommand = ViewCommand {
    name: "redo",
    id: "editor.redo",
    action: |_id, v, c| {
        v.doc.lock().redo();
        v.refocus_main_selection(c);
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
        #[cfg(target_os = "windows")]
        context.window_mut().winit().unwrap().set_enable(false);
        if let Some(file) = FileDialog::new().pick_file() {
            // TODO: check for errors
            let doc = Document::from_file(file).unwrap();
            w.add_new_doc(Dynamic::new(doc), context)
        }
        #[cfg(target_os = "windows")]
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

const PREVNEXT_DOC_ACTION: fn(WidgetId, &EditorWindow, &mut EventContext) = |_id, w, c| {
    let items = w
        .documents
        .get()
        .iter()
        .map(|d| d.get().title())
        .collect::<Vec<_>>();

    let mut v = w
        .mru_documents
        .get()
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect::<Vec<_>>();
    v.sort_by(|a, b| b.1.cmp(&a.1));

    let items = v.iter().map(|(k, _)| items[*k].clone()).collect::<Vec<_>>();
    let next_key = get_settings()
        .shortcuts
        .get("window.nextdoc")
        .unwrap()
        .clone();
    let prev_key = get_settings()
        .shortcuts
        .get("window.prevdoc")
        .unwrap()
        .clone();
    let current_doc = w.current_doc.clone();
    c.palette("select a document")
        .items(items)
        .next_key(next_key)
        .prev_key(prev_key)
        .selected_idx(1)
        .accept(move |_, i, val| {
            dbg!("Selected!", i, val);
            *current_doc.lock() = v[i].0;
        })
        .show();
};

const NEXT_DOC: WindowCommand = WindowCommand {
    name: "Next Document",
    id: "window.nextdoc",
    action: PREVNEXT_DOC_ACTION,
};
const PREV_DOC: WindowCommand = WindowCommand {
    name: "Next Document",
    id: "window.prevdoc",
    action: PREVNEXT_DOC_ACTION,
};
const SELECT_DOC: WindowCommand = WindowCommand {
    name: "Select Document",
    id: "window.select_doc",
    action: |_id, w, c| {
        let items = w.documents.get().iter().map(|d| d.get().title()).collect();
        let current_doc = w.current_doc.clone();
        c.palette("Select a document")
            .items(items)
            .accept(move |_, i, val| {
                dbg!("Selected!", i, val);
                *current_doc.lock() = i;
            })
            .show();
    },
};

const DUPLICATE_SELECTION_DOWN: ViewCommand = ViewCommand {
    name: "Duplicate Selection Down",
    id: "editor.duplicate_selection_down",
    action: |_id, v, c| {
        v.doc.lock().duplicate_selection(ndoc::MoveDirection::Down);
        v.refocus_main_selection(c);
    },
};
const DUPLICATE_SELECTION_UP: ViewCommand = ViewCommand {
    name: "Duplicate Selection Up",
    id: "editor.duplicate_selection_up",
    action: |_id, v, c| {
        v.doc.lock().duplicate_selection(ndoc::MoveDirection::Up);
        v.refocus_main_selection(c);
    },
};

const DUPLICATE_SELECTION: ViewCommand = ViewCommand {
    name: "Duplicate Selection",
    id: "editor.duplicate_selection",
    action: |_id, v, c| {
        if v.doc.get().selections.len() == 1 && v.doc.get().selections[0].is_empty() {
            let mut d = v.doc.lock();
            let pos = d.selections[0].head;
            d.select_word(pos);
        } else {
            v.doc.lock().duplicate_selection_for_selected_text();
        }
        v.refocus_main_selection(c);
    },
};

const CHANGE_THEME: WindowCommand = WindowCommand {
    name: "Change Theme",
    id: "window.change_theme",
    action: |_id, w, c| {
        let items: Vec<String> = ThemeSetRegistry::get().themes.keys().cloned().collect();
        dbg!("palette theme");
        let documents = w.documents.clone();
        c.palette("Choose theme")
            .items(items)
            .accept(move |_, _, val| {
                for doc in documents.get() {
                    doc.lock().update_theme(&val);
                    SETTINGS.lock().unwrap().theme.clone_from(&val);
                }
            })
            .show();
    },
};

const SHOW_ALL_COMMAND: WindowCommand = WindowCommand {
    name: "Show All Commands",
    id: "window.show_all_commands",
    action: |_id, w, c| {
        let mut items = w
            .cmd_reg
            .get()
            .view
            .values()
            .map(|v| (v.id, v.name))
            .collect::<Vec<_>>();
        items.extend(w.cmd_reg.get().window.values().map(|v| (v.id, v.name)));

        items.sort_by_key(|i| i.1);
        //TODO, put recent items in front

        let i = items
            .iter()
            .map(|(_id, name)| name.to_string())
            .collect::<Vec<_>>();

        let cmd_reg = w.cmd_reg.clone();

        c.palette("All Commands")
            .items(i)
            .accept(move |c, index, _| {
                if items[index].0.starts_with("editor.") {
                    let editor_id = {
                        let (current_code_editor_id, doc_id) = {
                            let wguard = c.widget().lock();
                            let w = wguard.downcast_ref::<EditorWindow>().unwrap();
                            (w.editor_switcher_id, w.current_doc().get().id())
                        };

                        let editor_switched_id = c
                            .for_other(&current_code_editor_id)
                            .unwrap()
                            .widget()
                            .lock()
                            .downcast_ref::<EditorSwitcher>()
                            .unwrap()
                            .editors[&doc_id]
                            .widget()
                            .id();
                        c.for_other(&editor_switched_id)
                            .unwrap()
                            .widget()
                            .lock()
                            .downcast_ref::<CodeEditor>()
                            .unwrap()
                            .editor_id
                    };

                    let mut editor_context = c.for_other(&editor_id).unwrap();
                    let t = unsafe {
                        let wguard = editor_context.widget().lock();
                        let t = wguard.downcast_ref::<TextEditor>().unwrap() as *const TextEditor;
                        t.as_ref().unwrap()
                    };
                    let cmd = *cmd_reg.get().view.get(dbg!(items[index].0)).unwrap();
                    (cmd.action)(editor_id, t, &mut editor_context);
                } else {
                    let w = unsafe {
                        let wguard = c.widget().lock();
                        let w =
                            wguard.downcast_ref::<EditorWindow>().unwrap() as *const EditorWindow;
                        w.as_ref().unwrap()
                    };
                    let cmd = *cmd_reg.get().window.get(items[index].0).unwrap();
                    (cmd.action)(c.widget().id(), w, c);
                }
            })
            .show();
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

impl Default for CommandsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let settings = get_settings(); // force load settings
    let theme = ThemePair::from_scheme(
        &ColorSchemeBuilder::new(ColorSource::new(177.3, 0.5))
            .build(),
    );

    let mut cmd_reg = CommandsRegistry::new();

    cmd_reg.window.insert(NEW_DOC.id, NEW_DOC);
    cmd_reg.window.insert(NEXT_DOC.id, NEXT_DOC);
    cmd_reg.window.insert(PREV_DOC.id, PREV_DOC);

    cmd_reg.view.insert(GOTO_LINE.id, GOTO_LINE);
    cmd_reg.view.insert(UNDO_CMD.id, UNDO_CMD);
    cmd_reg.view.insert(REDO_CMD.id, REDO_CMD);

    cmd_reg.view.insert(SAVE_DOC_CMD.id, SAVE_DOC_CMD);
    cmd_reg.window.insert(OPEN_DOC.id, OPEN_DOC);
    cmd_reg.window.insert(CLOSE_DOC.id, CLOSE_DOC);
    cmd_reg.window.insert(SELECT_DOC.id, SELECT_DOC);
    cmd_reg
        .view
        .insert(DUPLICATE_SELECTION_DOWN.id, DUPLICATE_SELECTION_DOWN);
    cmd_reg
        .view
        .insert(DUPLICATE_SELECTION_UP.id, DUPLICATE_SELECTION_UP);
    cmd_reg
        .view
        .insert(DUPLICATE_SELECTION.id, DUPLICATE_SELECTION);
    cmd_reg.window.insert(CHANGE_THEME.id, CHANGE_THEME);
    cmd_reg.window.insert(SHOW_ALL_COMMAND.id, SHOW_ALL_COMMAND);

    for (command_id, shortcut) in settings
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("editor."))
    {
        if let Some(cmd) = cmd_reg.view.get(command_id.as_str()) {
            cmd_reg.view_shortcut.insert(shortcut.clone(), *cmd);
        }
    }

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("window."))
    {
        if let Some(cmd) = cmd_reg.window.get(command_id.as_str()) {
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

    let (editor_tag, editor_id) = WidgetTag::new();
    let editor = EditorWindow::new(doc.clone(), cmd_reg.clone());

    let docs = editor.documents.clone();
    let cur_doc = editor.current_doc.clone();

    let mut win = editor
        .make_with_tag(editor_tag)
        .expand()
        .and(
            StatusBar::new(docs.clone(), cur_doc)
                .centered()
                .pad_by(Px::new(2)),
        )
        .into_rows()
        .gutter(Px::ZERO)
        .themed(theme)
        .with(&components::TextSize, Lp::points(10))
        .with(
            &components::CornerRadius,
            CornerRadii::from(Dimension::Lp(Lp::points(0))),
        )
        .with(&components::IntrinsicPadding, Dimension::Lp(Lp::points(3)))
        .into_window()
        .on_close_requested(move |()| {
            if !docs.get().iter().any(|d| d.get().is_dirty()) {
                return true;
            }
            palette("Unsaved changes, are you sure you want to close?")
                .owner(editor_id)
                .items(vec!["Yes".to_string(), "No".to_string()])
                .accept(|c, _, r| {
                    if let "Yes" = r.as_str() {
                        c.window_mut().close()
                    }
                })
                .show();
            false
        });

    win.title = Value::Constant("SomePad".to_string());
    win.attributes.min_inner_size = Some(Size::Logical(LogicalSize::new(800., 600.)));

    win.run()?;

    // TODO: Save settings

    Ok(())
}
