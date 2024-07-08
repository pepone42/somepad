use std::{default, hash::Hash, marker::PhantomData};

use floem::{
    action::{add_overlay, remove_overlay},
    id::Id,
    keyboard::{KeyEvent, Modifiers, ModifiersState, NamedKey},
    kurbo::{Point, Rect},
    peniko::Color,
    reactive::{create_effect, create_rw_signal, RwSignal},
    unit::Auto,
    view::View,
    views::{
        container, editor::text, empty, h_stack, label, v_stack, virtual_list, Decorators,
        VirtualDirection, VirtualItemSize,
    },
    widgets::text_input,
    Application,
};
use ndoc::Document;

use crate::{documents::Documents, editor, focused_editor, get_settings, text_editor};

use super::{get_id_path, WINDOWS_VIEWPORT};

pub trait PaletteItem<K: Clone + Copy> {
    fn id(&self) -> K;
    fn name(&self) -> String;
    fn description(&self) -> Option<String>;
}

impl PaletteItem<usize> for (usize, String) {
    fn id(&self) -> usize {
        self.0
    }
    fn name(&self) -> String {
        self.1.clone()
    }
    fn description(&self) -> Option<String> {
        None
    }
}

impl PaletteItem<usize> for (usize, String, String) {
    fn id(&self) -> usize {
        self.0
    }
    fn name(&self) -> String {
        self.1.clone()
    }
    fn description(&self) -> Option<String> {
        Some(self.2.clone())
    }
}

fn palette_free(owner_id: Id, action: impl FnOnce(String) + 'static + Clone + Copy) {
    if let Some(viewport) = WINDOWS_VIEWPORT.with(|w| {
        for id in get_id_path(owner_id) {
            if let Some(v) = w.borrow().get(&id) {
                return Some(v.clone());
            }
        }
        return None;
    }) {
        const PALETTE_WIDTH: f64 = 300.;

        let focused_id = focused_editor();

        add_overlay(Point::new(0., 0.), move |id| {
            id.request_focus();

            let doc = create_rw_signal(Document::new(get_settings().indentation));

            container(
                container(
                    text_editor(move || doc)
                        .multiline(false)
                        .on_return(move || {
                            if doc.get().rope.len_chars() > 0 {
                                action(doc.get().rope.to_string());
                            }
                            focused_id.request_focus();
                            remove_overlay(id);
                        })
                        .on_escape(move || {
                            remove_overlay(id);
                            focused_id.request_focus();
                        }),
                )
                .style(move |s| {
                    s.flex()
                        .margin_bottom(Auto)
                        .width(PALETTE_WIDTH)
                        .background(Color::DARK_BLUE)
                }),
            )
            .style(move |s| {
                s.flex()
                    .justify_center()
                    .size(viewport.get().width(), viewport.get().height())
            })
            .on_click_stop(move |_| {
                remove_overlay(id);
                focused_id.request_focus();
            })
        });
    } else {
        //log error
    }
}

fn palette_list(
    owner_id: Id,
    items: im::Vector<(usize, String)>,
    action: impl FnOnce(usize) + 'static + Clone + Copy,
) {
    if let Some(viewport) = WINDOWS_VIEWPORT.with(|w| {
        for id in get_id_path(owner_id) {
            if let Some(v) = w.borrow().get(&id) {
                return Some(v.clone());
            }
        }
        return None;
    }) {
        const PALETTE_WIDTH: f64 = 300.;

        let current = create_rw_signal(0);
        let current_key = create_rw_signal(0);
        let focused_id = focused_editor();

        add_overlay(Point::new(0., 0.), move |id| {
            id.request_focus();

            let doc = create_rw_signal(Document::new(get_settings().indentation));
            let sorted_items = create_rw_signal(items.clone());
            create_effect(move |_| {
                sorted_items.set(
                    items
                        .iter()
                        .filter(|(_, name)| name.contains(&doc.get().rope.to_string()))
                        .map(|(id, name)| (id.clone(), name.clone()))
                        .collect(),
                );
                current.set(0);
            });
            create_effect(move |_| {
                if sorted_items.get().len() == 0 {
                    return;
                }
                if current.get() >= sorted_items.get().len() {
                    current.set(sorted_items.get().len() - 1);
                }
                current_key.set(sorted_items.get()[current.get()].0.clone());
            });

            container(
                v_stack((
                    text_editor(move || doc)
                        .multiline(false)
                        .on_arrow_up(move || {
                            if current.get() > 0 {
                                current.set(current.get() - 1);
                            }
                        })
                        .on_arrow_down(move || {
                            if current.get() < sorted_items.get().len() - 1 {
                                current.set(current.get() + 1);
                            }
                        })
                        .on_return(move || {
                            if !sorted_items.get().is_empty() {
                                action(current_key.get());
                            }
                            focused_id.request_focus();
                            remove_overlay(id);
                        })
                        .on_escape(move || {
                            focused_id.request_focus();
                            remove_overlay(id);
                        }),
                    //.style(|s| s.flex_grow(1.0)),
                    empty().style(|s| s.border(1.0).color(Color::BLACK)),
                    virtual_list(
                        VirtualDirection::Vertical,
                        VirtualItemSize::Fixed(Box::new(|| 20.0)),
                        move || sorted_items.get().clone(),
                        move |item: &(usize, String)| item.clone(),
                        move |item| {
                            let key1 = item.0.clone();
                            let key2 = item.0.clone();
                            label(move || item.1.to_string())
                                .on_click_stop(move |_| {
                                    action(key1);
                                    remove_overlay(id);
                                    focused_id.request_focus();
                                })
                                .style(move |s| {
                                    if current_key.get() == key2 {
                                        s.background(Color::SKY_BLUE)
                                    } else {
                                        s.background(Color::DARK_BLUE)
                                    }
                                    .width(PALETTE_WIDTH)
                                })
                        },
                    ),
                ))
                .style(move |s| {
                    s.flex()
                        .margin_bottom(Auto)
                        .width(PALETTE_WIDTH)
                        .background(Color::DARK_BLUE)
                }),
            )
            .style(move |s| {
                s.flex()
                    .justify_center()
                    .size(viewport.get().width(), viewport.get().height())
            })
            .on_click_stop(move |_| {
                remove_overlay(id);
                focused_id.request_focus();
            })
        });
    } else {
        //log error
    }
}

pub struct PaletteBuilder {
    owner_id: Id,
    description: Option<String>,
}

pub struct PaletteListBuilder {
    owner_id: Id,
    description: Option<String>,
    items: im::Vector<(usize, String)>,
}



impl PaletteBuilder {
    pub fn new(owner_id: Id) -> Self {
        Self {
            owner_id,
            description: None,
        }
    }

    pub fn description(mut self, description: impl ToString) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn items(self, items: im::Vector<(usize, String)>) -> PaletteListBuilder {
        PaletteListBuilder {
            owner_id: self.owner_id,
            description: self.description,
            items: im::Vector::from_iter(items),
        }
    }
    pub fn build(self, action: impl FnOnce(String) + 'static + Clone + Copy) {
        palette_free(self.owner_id, action);
    }
}

impl PaletteListBuilder {
    pub fn build(self, action: impl FnOnce(usize) + 'static + Clone + Copy) {
        palette_list(self.owner_id, self.items, action);
    }
}
