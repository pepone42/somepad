use std::fmt::Debug;

use cushy::{
    figures::{
        units::{Px, UPx},
        IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero,
    },
    kludgine::{shapes::Shape, text::Text, DrawableExt},
    styles::components,
    value::{Dynamic, DynamicReader, Source},
    widget::{Widget, HANDLED},
    ConstraintLimit, WithClone,
};

use crate::widgets::palette::PaletteAction;

use super::palette::{close_palette, PALETTE_STATE};

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

        let filtered_items = (&selected_idx, &items)
            .with_clone(|(selected_idx, items)| {
                filter.map_each(move |filter| {
                    if !filter.is_empty() {
                        for item in items.lock().iter_mut() {
                            item.excluded = !item.text.contains(filter);
                        }
                        if let Some(i) = items.get().iter().filter(|i| !i.excluded).nth(0) {
                            *selected_idx.lock() = Some(i.index);
                        } else {
                            *selected_idx.lock() = None;
                        }
                    }
                    items
                        .get()
                        .iter()
                        .filter(|i| !i.excluded)
                        .cloned()
                        .collect::<Vec<FilterItem>>()
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
            selected_idx: dbg!(selected_idx),
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

pub struct FilteredList {
    pub filter: Dynamic<Filter>,
    pub hovered_idx: Dynamic<Option<usize>>,
    action: Dynamic<PaletteAction>,
}

impl Debug for FilteredList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilteredList")
            .field("filter", &self.filter)
            .field("hovered_idx", &self.hovered_idx)
            .field("action", &"self.action")
            .finish()
    }
}

impl FilteredList {
    pub fn new(
        items: Vec<String>,
        filter: Dynamic<String>,
        selected_idx: usize,
        action: Dynamic<PaletteAction>,
    ) -> Self {
        let filter = Dynamic::new(Filter::new(items, filter, selected_idx));
        FilteredList {
            filter,
            hovered_idx: Dynamic::new(None),
            action,
        }
    }
}

impl Widget for FilteredList {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.gfx.scale())
            .round();

        context.apply_current_font_settings();
        context.redraw_when_changed(&self.filter);

        let surface_color = context.get(&components::WidgetBackground);
        context.gfx.fill(surface_color);

        let scale = context.gfx.scale();
        let size = context.gfx.size();
        let mut y = Px::ZERO;
        let selected_idx = self.filter.get().selected_idx.get();
        let line_height = context.gfx.line_height().into_upx(scale);
        let bg_selected_color = context.get(&components::DefaultActiveBackgroundColor);
        let bg_hovered_color = context.get(&components::DefaultHoveredBackgroundColor);
        let fg_selected_color = context.get(&components::DefaultActiveForegroundColor);
        let fg_hovered_color = context.get(&components::DefaultHoveredForegroundColor);
        let fg_color = context.get(&components::TextColor);

        if let Some(idx) = self.hovered_idx.get() {
            context.gfx.draw_shape(
                Shape::filled_rect(
                    Rect::new(
                        Point::ZERO,
                        Size::new(
                            size.width.into_signed() - padding * 2,
                            line_height.into_signed(),
                        ),
                    ),
                    bg_hovered_color,
                )
                .translate_by(Point::new(
                    padding,
                    line_height.into_signed() * Px::new(idx as i32) + padding,
                )),
            )
        }

        for item in self.filter.get().filtered_items.get().iter() {
            if selected_idx == Some(item.index) {
                context.gfx.draw_shape(
                    Shape::filled_rect(
                        Rect::new(
                            Point::ZERO,
                            Size::new(
                                size.width.into_signed() - padding * 2,
                                line_height.into_signed(),
                            ),
                        ),
                        bg_selected_color,
                    )
                    .translate_by(Point::new(padding, y + padding)),
                )
            }
            let color = if selected_idx == Some(item.index) {
                fg_selected_color
            } else if self.hovered_idx.get() == Some(item.index) {
                fg_hovered_color
            } else {
                fg_color
            };
            let text = Text::new(&item.text, color);
            let h = line_height.into_signed();
            context
                .gfx
                .draw_text(text.translate_by(Point::new(Px::ZERO + padding, y + padding)));

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
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_upx(context.gfx.scale())
            .round()
            * 2;
        context.apply_current_font_settings();

        let mut y = UPx::ZERO;
        let mut w = UPx::ZERO;
        for item in self.filter.get().filtered_items.get().iter() {
            //let text = format!("{}*", item.text);
            let text = Text::<UPx>::new(&item.text, cushy::kludgine::Color::WHITE);
            let h = context.gfx.measure_text(text).line_height;
            y += h;
            if context.gfx.measure_text(text).size.width > w {
                w = context.gfx.measure_text(text).size.width;
            }
        }
        let w = if let ConstraintLimit::Fill(w) = _available_space.width {
            w
        } else {
            w
        };
        Size::new(w, y + padding)
    }
    fn hit_test(
        &mut self,
        _location: Point<Px>,
        _context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        _button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let location = location - padding;
        let scale = context.kludgine.scale();
        let line_height = context.kludgine.line_height().into_px(scale);
        let idx = (location.y / line_height).floor().get();
        *self.filter.get().selected_idx.lock() = Some(idx as usize);
        if let Some(item) = self.filter.get().selected_item.get() {
            close_palette();
            self.action.get()(
                &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                idx as usize,
                item.text.clone(),
            );
        }

        HANDLED
    }

    fn hover(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> Option<cushy::kludgine::app::winit::window::CursorIcon> {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let location = location - padding;
        context.redraw_when_changed(&self.hovered_idx);
        let scale = context.kludgine.scale();
        let line_height = context.kludgine.line_height().into_px(scale);

        let idx = (location.y / line_height).floor().get();

        *self.hovered_idx.lock() = Some(idx as usize);

        None
    }

    fn unhover(&mut self, context: &mut cushy::context::EventContext<'_>) {
        context.redraw_when_changed(&self.hovered_idx);
        *self.hovered_idx.lock() = None;
    }
}
