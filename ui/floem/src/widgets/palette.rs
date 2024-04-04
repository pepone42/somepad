use floem::{
    action::{add_overlay, remove_overlay},
    id::Id,
    keyboard::{KeyEvent, ModifiersState, NamedKey},
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

use crate::{documents::Documents, editor, text_editor};

pub trait PaletteItem {
    fn id(&self) -> usize;
    fn name(&self) -> String;
    fn description(&self) -> Option<String>;
}

impl PaletteItem for (usize, String) {
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

impl PaletteItem for (usize, String, String) {
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

pub fn palette(
    viewport: Rect,
    items: impl Iterator<Item = (usize, String)>,
    action: impl FnOnce(usize) + 'static + Clone + Copy,
) {
    let items = im::Vector::from_iter(items);
    const PALETTE_WIDTH: f64 = 300.;

    let current = create_rw_signal(0);
    let current_key = create_rw_signal(0);

    add_overlay(Point::new(0., 0.), move |id| {
        id.request_focus();
        let doc = create_rw_signal(Document::default());
        let sorted_items = create_rw_signal(items.clone());
        create_effect(move |_| {
            sorted_items.set(
                items
                    .iter()
                    .filter(|(_, name)| name.contains(&doc.get().rope.to_string()))
                    .map(|(id, name)| (*id, name.clone()))
                    .collect(),
            );
            current.set(0);
        });
        create_effect(move |_| {
            if sorted_items.get().len() == 0 {
                return;
            }
            current_key.set(sorted_items.get()[current.get()].0);
        });

        container(
            v_stack((
                text_editor(create_rw_signal(Documents::new()), move || doc.clone())
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
                            action(sorted_items.get()[current.get()].0);
                        }
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
                        label(move || item.1.to_string())
                            .on_click_stop(move |_| {
                                action(item.0);
                                remove_overlay(id);
                            })
                            .style(move |s| {
                                if current_key.get() == item.0 {
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
                .size(viewport.width(), viewport.height())
        })
        .on_click_stop(move |_| remove_overlay(id))
    });
}
