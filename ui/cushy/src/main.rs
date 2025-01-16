#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[macro_use]
mod shortcut;
mod settings;
mod utils;
mod widgets;

use cushy::context::EventContext;
use cushy::figures::{Size, Zero};
#[cfg(windows)]
use cushy::kludgine::app::winit::platform::windows::WindowExtWindows;
use cushy::kludgine::wgpu::naga::proc::index::GuardedIndex;
use cushy::widgets::layers::Modal;
use ndoc::syntax::ThemeSetRegistry;
use rfd::FileDialog;
use widgets::editor_switcher::EditorSwitcher;
use widgets::editor_window::EditorWindow;
use widgets::input::Input;
use utils::DowncastWidget;
use widgets::palette::{Palette, PaletteState};
use widgets::status_bar::StatusBar;
use widgets::text_editor::{self, CodeEditor, TextEditor};

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use cushy::figures::units::{Lp, Px, UPx};

use cushy::kludgine::cosmic_text::FontSystem;
use cushy::styles::components;
use cushy::styles::{
    ColorSchemeBuilder, ColorSource, CornerRadii, Dimension, FamilyOwned, FontFamilyList, ThemePair,
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
        tracing::trace!("Add New Document");
        w.add_new_doc(Dynamic::new(Document::default()), c);
    },
};

const GOTO_LINE: ViewCommand = ViewCommand {
    name: "Go to Line",
    id: "editor.goto_line",
    action: |_id, v, _c| {
        let doc = v.doc.clone();
        v.palette()
            .description("Got to line")
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
            *w.current_doc.lock() = w.current_doc.get().saturating_sub(1);
        }
        // TODO: close the window if there is only one doc
        // TODO: warn if the doc is dirty
    },
};

const PREVNEXT_DOC_ACTION: fn(WidgetId, &EditorWindow, &mut EventContext) = |_id, w, _c| {
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
    w.palette()
        .description("select a document")
        .items(items)
        .next_key(next_key)
        .prev_key(prev_key)
        .selected_idx(1)
        .accept(move |_, i, _| {
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
    name: "Prev Document",
    id: "window.prevdoc",
    action: PREVNEXT_DOC_ACTION,
};
const SELECT_DOC: WindowCommand = WindowCommand {
    name: "Select Document",
    id: "window.select_doc",
    action: |_id, w, _c| {
        let items = w.documents.get().iter().map(|d| d.get().title()).collect();
        let current_doc = w.current_doc.clone();
        w.palette()
            .description("Select a document")
            .items(items)
            .accept(move |_, i, _| {
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

const TOGGLE_SEARCH_PANEL: ViewCommand = ViewCommand {
    name: "Show Search Panel",
    id: "editor.show_search_panel",
    action: |_id, v, c| {
        v.toggle_search_panel(c);
    },
};

const CHANGE_THEME: WindowCommand = WindowCommand {
    name: "Change Theme",
    id: "window.change_theme",
    action: |_id, w, _c| {
        let items: Vec<String> = ThemeSetRegistry::get().themes.keys().cloned().collect();
        let documents = w.documents.clone();
        w.palette()
            .description("Choose theme")
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

const CHANGE_LANGUAGE: ViewCommand = ViewCommand {
    name: "Change Language",
    id: "editor.change_language",
    action: |_id, v, _c| {
        let languages: Vec<String> = ndoc::syntax::SYNTAXSET
            .syntaxes()
            .iter()
            .map(|l| l.name.clone())
            .collect();
        let doc = v.doc.clone();
        v.palette()
            .description("Choose language")
            .items(languages)
            .accept(move |_, _, val| {
                doc.lock().update_language(&val);
            })
            .show();
    },
};

const SHOW_ALL_COMMAND: WindowCommand = WindowCommand {
    name: "Show All Commands",
    id: "window.show_all_commands",
    action: |_id, w, _c| {
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

        let switcher = w.editor_switcher.clone();
        let editor_window = _c.widget().instance().clone();

        w.palette()
            .description("All Commands")
            .items(i)
            .accept(move |c, index, _| {
                if items[index].0.starts_with("editor.") {
                    let text_editor_id = switcher.use_as(move |f: &EditorSwitcher| {
                        f.current_editor()
                            .use_as(move |text_editor: &TextEditor| text_editor.id.unwrap())
                    });

                    let mut editor_context = c.for_other(&text_editor_id).expect("editor context");

                    let cmd = *cmd_reg.get().view.get(items[index].0).unwrap();
                    switcher.use_as(|f: &EditorSwitcher| {
                        f.current_editor().use_as(|text_editor: &TextEditor| {
                            (cmd.action)(text_editor_id, text_editor, &mut editor_context);
                        })
                    });
                } else {
                    let cmd = *cmd_reg.get().window.get(items[index].0).unwrap();
                    editor_window.use_as(|w: &EditorWindow| {
                        (cmd.action)(w.id.unwrap(), w, c);
                    });
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

impl CommandsRegistry {
    pub fn register() -> Self {
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
        cmd_reg
            .view
            .insert(TOGGLE_SEARCH_PANEL.id, TOGGLE_SEARCH_PANEL);
        cmd_reg.window.insert(CHANGE_THEME.id, CHANGE_THEME);
        cmd_reg.view.insert(CHANGE_LANGUAGE.id, CHANGE_LANGUAGE);
        cmd_reg.window.insert(SHOW_ALL_COMMAND.id, SHOW_ALL_COMMAND);
        cmd_reg
    }

    fn bind_shortcuts(&mut self, settings: Settings) {
        for (command_id, shortcut) in settings
            .shortcuts
            .iter()
            .filter(|(id, _)| id.starts_with("editor."))
        {
            if let Some(cmd) = self.view.get(command_id.as_str()) {
                self.view_shortcut.insert(shortcut.clone(), *cmd);
            }
        }

        for (command_id, shortcut) in settings
            .shortcuts
            .iter()
            .filter(|(id, _)| id.starts_with("window."))
        {
            if let Some(cmd) = self.window.get(command_id.as_str()) {
                self.window_shortcut.insert(shortcut.clone(), *cmd);
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    #[cfg(target_os = "windows")]
    let ui_font = FamilyOwned::Name("Segoe UI".to_string());
    #[cfg(not(target_os = "windows"))]
    let ui_font = FamilyOwned::SansSerif;

    let settings = get_settings(); // force load settings
    let theme = ThemePair::from_scheme(
        &ColorSchemeBuilder::new(ColorSource::new(-96.8, 0.1))
            .neutral(ColorSource::new(-126.9, 0.2))
            .build(),
    );

    let mut cmd_reg = CommandsRegistry::register();
    cmd_reg.bind_shortcuts(settings);

    let cmd_reg = Dynamic::new(cmd_reg);
    let modal = Modal::new();

    ndoc::Document::init_highlighter();
    let doc = Dynamic::new(if let Some(path) = std::env::args().nth(1) {
        if !Path::new(&path).exists() {
            File::create_new(&path)?;
        }
        ndoc::Document::from_file(path)?
    } else {
        ndoc::Document::default()
    });

    let (editor_tag, editor_id) = WidgetTag::new();
    let editor = EditorWindow::new(doc.clone(), cmd_reg.clone(), modal.clone());

    let docs = editor.documents.clone();
    let cur_doc = editor.current_doc.clone();

    let mut win = editor
        .make_with_tag(editor_tag)
        .expand()
        .and(
            StatusBar::new(docs.clone(), cur_doc)
                .centered()
                .pad_by(Px::new(2))
                .and(Input::new(Dynamic::new("hello \n world".to_string())).height(Lp::cm(1))).into_rows(),
        )
        .into_rows()
        .gutter(Px::ZERO)
        .and(modal.clone())
        .into_layers()
        .themed(theme)
        .with(&components::BaseTextSize, Lp::points(9))
        .with(&components::FontFamily, FontFamilyList::from(ui_font))
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
            let m = modal.clone();
            modal.present(Palette::new(
                PaletteState::new(m)
                    .description("Unsaved changes, are you sure you want to close?")
                    .owner(editor_id)
                    .items(vec!["Yes".to_string(), "No".to_string()])
                    .accept(|c, _, r| {
                        if let "Yes" = r.as_str() {
                            c.window_mut().close()
                        }
                    }),
            ));
            false
        });

    win.title = Value::Constant("SomePad".into());
    let inner_size = Dynamic::new(Size::new(UPx::new(800), UPx::new(600)));

    win.inner_size(inner_size).run()?;

    // TODO: Save settings

    Ok(())
}
