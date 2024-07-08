use cushy::{
    figures::{
        units::{Px, UPx},
        Point, Rect, ScreenScale, Size, Zero,
    },
    kludgine::{
        shapes::{Shape, StrokeOptions},
        text::Text,
        DrawableExt,
    },
    styles::Color,
    value::{Dynamic, DynamicReader, Source},
    widget::Widget,
};

#[derive(Debug, Clone, PartialEq)]
pub struct FilterItem {
    pub index: usize,
    score: usize,
    pub text: String,
    excluded: bool,
}

#[derive(Debug, Clone)]
pub struct Filter {
    items: Dynamic<Vec<FilterItem>>,
    pub selected_idx: Dynamic<Option<usize>>,
    pub selected_item: DynamicReader<Option<FilterItem>>,
    pub filtered_items: DynamicReader<Vec<FilterItem>>,
}

impl Filter {
    pub fn new(items: Vec<String>, filter: Dynamic<String>, selected_idx: usize) -> Self {
        let mut init = true;
        let items: Dynamic<Vec<FilterItem>> = Dynamic::new(
            items
                .into_iter()
                .enumerate()
                .map(|(index, text)| FilterItem {
                    index,
                    score: 0,
                    text,
                    excluded: false,
                })
                .collect(),
        );

        let selected_idx = if items.get().is_empty() {
            Dynamic::new(None)
        } else {
            Dynamic::new(Some(selected_idx.min(items.get().len() - 1)))
        };

        let filtered_items = selected_idx
            .with_clone(|selected_idx| {
                items.with_clone(|items| {
                    filter.map_each(move |filter| {
                        if !init {
                            for item in items.lock().iter_mut() {
                                item.excluded = !item.text.contains(filter);
                            }
                            if let Some(i) = items.get().iter().filter(|i| !i.excluded).nth(0) {
                                *selected_idx.lock() = Some(i.index);
                            } else {
                                *selected_idx.lock() = None;
                            }
                        }
                        init = false;
                        items
                            .get()
                            .iter()
                            .filter(|i| !i.excluded)
                            .cloned()
                            .collect::<Vec<FilterItem>>()
                    })
                })
            })
            .into_reader();

        let selected_item = items
            .with_clone(|items| {
                selected_idx.map_each(move |selected_idx| {
                    selected_idx.and_then(|s| Some(items.get()[s].clone()))
                })
            })
            .into_reader();

        Filter {
            items,
            selected_idx,
            selected_item,
            filtered_items,
        }
    }

    pub fn next(&mut self) {
        let items = self.items.get();
        let mut idx = self.selected_idx.get();
        if idx.is_none() {
            return;
        }
        loop {
            let i = idx.unwrap();
            idx = Some((i + 1) % items.len());
            if !items[idx.unwrap()].excluded {
                break;
            }
        }
        *self.selected_idx.lock() = idx;
    }
    pub fn prev(&mut self) {
        let items = self.items.get();
        let idx = self.selected_idx.clone();
        if idx.get().is_none() {
            return;
        }
        loop {
            let i = idx.get().unwrap();
            *idx.lock() = Some((i + items.len() - 1) % items.len());
            if !items[idx.get().unwrap()].excluded {
                break;
            }
        }
    }
}

#[derive(Debug)]
pub struct FilteredList {
    pub filter: Dynamic<Filter>,
}

impl FilteredList {
    pub fn new(items: Vec<String>, filter: Dynamic<String>, selected_idx: usize) -> Self {
        let filter = Dynamic::new(Filter::new(items, filter, selected_idx));
        FilteredList { filter }
    }
}

impl Widget for FilteredList {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        context.apply_current_font_settings();
        context.redraw_when_changed(&self.filter);
        let mut y = Px::ZERO;
        let selected_idx = self.filter.get().selected_idx.get();
        for item in self.filter.get().filtered_items.get().iter() {
            let text = format!(
                "{}{}",
                item.text,
                if selected_idx == Some(item.index) {
                    "*"
                } else {
                    ""
                }
            );
            let text = Text::<Px>::new(&text, cushy::kludgine::Color::WHITE);
            let h = context.gfx.measure_text(text).line_height;
            context
                .gfx
                .draw_text(text.translate_by(Point::new(Px::ZERO, y)));
            y += h;
        }

        // let line_height = context.gfx.line_height().into_px(context.gfx.scale());
        // if let Some(idx) = self.filter.get().selected_idx.get() {
        //     let y = line_height * Px::new(idx as i32);
        //     context.gfx.draw_shape(
        //         Shape::stroked_rect(
        //             Rect::new(
        //                 Point::new(Px::ZERO, y - (line_height)),
        //                 Size::new(Px::ZERO, line_height * 4),
        //             ),
        //             StrokeOptions::default().colored(Color::WHITE),
        //         )
        //         .translate_by(Point::ZERO),
        //     )
        // }
    }
    fn layout(
        &mut self,
        _available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<cushy::figures::units::UPx> {
        context.apply_current_font_settings();
        let mut y = UPx::ZERO;
        let mut w = UPx::ZERO;
        for item in self.filter.get().filtered_items.get().iter() {
            let text = format!("{}*", item.text);
            let text = Text::<UPx>::new(&text, cushy::kludgine::Color::WHITE);
            let h = context.gfx.measure_text(text).line_height;
            y += h;
            if context.gfx.measure_text(text).size.width > w {
                w = context.gfx.measure_text(text).size.width;
            }
        }
        Size::new(w, y)
    }
    fn hit_test(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
    }
}
