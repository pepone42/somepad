
use cushy::{
    figures::{
        units::{Px, UPx},
        Point, Size, Zero,
    },
    kludgine::{text::Text, DrawableExt},
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
    selected_idx: Dynamic<Option<usize>>,
    pub selected_item: DynamicReader<Option<FilterItem>>,
    pub filtered_items: DynamicReader<Vec<FilterItem>>,
}

impl Filter {
    pub fn new(items: Vec<String>, filter: Dynamic<String>) -> Self {
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
        let selected_idx = Dynamic::new(Some(0));
        let filtered_items = selected_idx.with_clone(|selected_idx| items.with_clone(|items| {
            filter.map_each(move|filter| {
                for item in items.lock().iter_mut() {
                    if !item.text.contains(filter) {
                        item.excluded = true;
                    } else {
                        item.excluded = false;
                    }
                }
                if let Some(i) = items.get().iter().filter(|i| !i.excluded).nth(0) {
                    *selected_idx.lock() =  Some(i.index);
                } else {
                    *selected_idx.lock() =  None;
                }
                items
                    .get()
                    .iter()
                    .filter(|i| !i.excluded)
                    .map(|i| i.clone())
                    .collect::<Vec<FilterItem>>()
            })
        })).into_reader();
        
        let selected_item = items.with_clone(|items| {
            selected_idx.map_each(move |selected_idx| selected_idx.and_then(|s| Some(items.get()[s].clone())))
        }).into_reader();

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
        *self.selected_idx.lock() = dbg!(idx);
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
    pub fn new(items: Vec<String>, filter: Dynamic<String>) -> Self {
        let filter = Dynamic::new(Filter::new(items, filter));
        FilteredList { filter }
    }
}

impl Widget for FilteredList {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        context.apply_current_font_settings();
        context.redraw_when_changed(&self.filter);
        let mut y = Px::ZERO;
        let selected_idx = dbg!(self.filter.get().selected_idx.get());
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
    }
    fn layout(
        &mut self,
        available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<cushy::figures::units::UPx> {
        context.apply_current_font_settings();
        let mut y = UPx::ZERO;
        for item in self.filter.get().filtered_items.get().iter() {
            let text = Text::<UPx>::new(&item.text, cushy::kludgine::Color::WHITE);
            let h = context.gfx.measure_text(text).line_height;
            y += h;
        }
        Size::new(available_space.width.max() - 1, y)
    }
}
