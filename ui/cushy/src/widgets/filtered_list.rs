use std::collections::{BTreeMap, HashMap};

use cushy::{
    figures::{
        units::{Px, UPx},
        Point, Size, Zero,
    },
    kludgine::{text::Text, wgpu::core::id, DrawableExt},
    styles::components::LineHeight,
    value::{Dynamic, Source},
    widget::{Widget, IGNORED},
};

#[derive(Debug)]
pub struct FilteredList {
    pub items: Vec<String>,
    filter: Dynamic<String>,
    pub filtered_items: Dynamic<BTreeMap<usize, String>>,
    pub filtered_item_idx: Dynamic<Option<usize>>,
    pub selected_item: Dynamic<Option<(usize, String)>>,
}

impl FilteredList {
    pub fn new(items: Vec<String>, filter: Dynamic<String>) -> Self {
        // TODO: maybe use im
        let i = items.clone();
        let filtered_items: Dynamic<BTreeMap<usize, String>> = filter.map_each(move |filter| {
            i.iter()
                .enumerate()
                .filter(|(_, item)| item.contains(filter))
                .map(|(i, item)| (i, item.clone()))
                .collect()
        });
        let filtered_item_idx = filtered_items
            .with_clone(|fi| fi.map_each(|items| if items.len() > 0 { Some(0) } else { None }));
        let selected_item = filtered_items.with_clone(|fi| {
            filtered_item_idx.map_each(move |idx| {
                if let Some(idx) = idx {
                    let items = fi.get();
                    Some((*idx, items.get(&idx).unwrap().clone()))
                } else {
                    None
                }
            })
        });

        FilteredList {
            items,
            filter,
            filtered_items,
            filtered_item_idx,
            selected_item,
        }
    }
}

impl Widget for FilteredList {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        context.apply_current_font_settings();
        let mut y = Px::ZERO;
        for item in self.filtered_items.get().iter().enumerate() {
            let text = format!(
                "{}{}",
                item.1 .1,
                match self.filtered_item_idx.get() {
                    Some(x) if x == item.0 => "*",
                    _ => "",
                }
            );
            let text = Text::<Px>::new(&text, cushy::kludgine::Color::WHITE);
            let h = context.gfx.measure_text(text).line_height;
            context
                .gfx
                .draw_text(text.translate_by(Point::new(Px::ZERO, y)));
            y += h;
        }
    }
    fn layout(
        &mut self,
        available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<cushy::figures::units::UPx> {
        context.apply_current_font_settings();
        let mut y = UPx::ZERO;
        for item in self.filtered_items.get().iter() {
            let text = Text::<UPx>::new(item.1, cushy::kludgine::Color::WHITE);
            let h = context.gfx.measure_text(text).line_height;
            y += h;
        }
        Size::new(available_space.width.max() - 1, y)
    }
}
