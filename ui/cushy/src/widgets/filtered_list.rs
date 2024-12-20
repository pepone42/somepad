use std::fmt::Debug;

use cushy::{
    figures::{
        units::{Px, UPx},
        IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero,
    },
    kludgine::{shapes::Shape, text::Text, DrawableExt},
    styles::components,
    value::{Dynamic, DynamicReader, Source},
    widget::{Widget, WidgetId, HANDLED},
    widgets::layers::Modal,
    ConstraintLimit, WithClone,
};
use sublime_fuzzy::best_match;

use crate::widgets::palette::PaletteAction;

#[derive(Debug, Clone, PartialEq)]
pub struct FilterItem {
    pub index: usize,
    score: isize,
    pub text: String,
    excluded: bool,
}

#[derive(Debug, Clone)]
pub struct Filter {
    pub selected_idx: Dynamic<Option<usize>>,
    pub selected_item: DynamicReader<Option<FilterItem>>,
    pub filtered_items: DynamicReader<Vec<FilterItem>>,
}

impl Filter {
    pub fn new(items: Vec<String>, filter: Dynamic<String>, initial_selected_idx: usize) -> Self {
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

        let initial_selected_idx = initial_selected_idx.min(items.get().len().saturating_sub(1));

        let selected_idx = if items.get().is_empty() {
            Dynamic::new(None)
        } else {
            Dynamic::new(Some(initial_selected_idx))
        };

        let filtered_items = (&selected_idx, &items)
            .with_clone(|(selected_idx, items)| {
                filter.map_each(move |filter| {
                    if filter.is_empty() {
                        items.get()
                    } else {
                        for item in items.lock().iter_mut() {
                            let search_match = best_match(filter, &item.text);
                            if let Some(search_match) = search_match {
                                item.score = search_match.score();
                                item.excluded = false;
                            } else {
                                item.score = 0;
                                item.excluded = true;
                            }
                        }
                        if let Some(i) = items
                            .get()
                            .iter()
                            .filter(|i| !i.excluded)
                            .enumerate()
                            .nth(initial_selected_idx)
                        {
                            *selected_idx.lock() = Some(i.0);
                        } else {
                            *selected_idx.lock() = None;
                        }

                        let mut items = items
                            .get()
                            .iter()
                            .filter(|i| !i.excluded)
                            .cloned()
                            .collect::<Vec<FilterItem>>();
                        items.sort_by(|a, b| b.score.cmp(&a.score));
                        items
                    }
                })
            })
            .into_reader();

        let selected_item = selected_idx
            .map_each({
                let filtered_items = filtered_items.clone();
                move |selected_idx| selected_idx.and_then(|s| filtered_items.get().get(s).cloned())
            })
            .into_reader();

        Filter {
            selected_idx,
            selected_item,
            filtered_items,
        }
    }

    pub fn next(&mut self) {
        let items = self.filtered_items.get();
        let mut idx = self.selected_idx.get();
        if idx.is_none() {
            return;
        }

        let i = idx.unwrap();
        idx = Some((i + 1) % items.len());

        *self.selected_idx.lock() = idx;
    }
    pub fn prev(&mut self) {
        let items = self.filtered_items.get();
        let idx = self.selected_idx.clone();
        if idx.get().is_none() {
            return;
        }

        let i = idx.get().unwrap();
        *idx.lock() = Some((i + items.len() - 1) % items.len());
    }
}

pub struct FilteredList {
    pub filter: Dynamic<Filter>,
    pub hovered_idx: Dynamic<Option<usize>>,
    action: PaletteAction,
    owner_id: WidgetId,
    modal: Modal,
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
        action: PaletteAction,
        owner_id: WidgetId,
        modal: Modal,
    ) -> Self {
        let filter = Dynamic::new(Filter::new(items, filter, selected_idx));
        FilteredList {
            filter,
            hovered_idx: Dynamic::new(None),
            action,
            owner_id,
            modal,
        }
    }
}

impl Widget for FilteredList {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        context.redraw_when_changed(&self.hovered_idx);
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.gfx.scale())
            .round();

        context.redraw_when_changed(&self.filter);

        let surface_color = context.get(&components::WidgetBackground);
        context.gfx.fill(surface_color);

        let scale = context.gfx.scale();
        let size = context.gfx.size();
        let mut y = Px::ZERO;
        let selected_idx = self.filter.get().selected_idx.get();
        let line_height = context.gfx.line_height().into_upx(scale);
        let bg_selected_color = context.get(&components::DefaultHoveredBackgroundColor);
        let fg_selected_color = context.get(&components::DefaultHoveredForegroundColor);

        let bg_hovered_color = context.get(&components::DefaultActiveBackgroundColor);
        let fg_hovered_color = context.get(&components::DefaultActiveForegroundColor);
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

        for (i,item) in self.filter.get().filtered_items.get().iter().enumerate() {
            if selected_idx == Some(i) {
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
            let color = if selected_idx == Some(i) {
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

        let mut w = UPx::ZERO;
        let line_height = context.gfx.line_height().into_upx(context.gfx.scale());
        let nb_item = self.filter.get().filtered_items.get().len();
        let height = UPx::new(nb_item as u32) * line_height + padding;

        // TODO: cache the width
        for item in self.filter.get().filtered_items.get().iter() {
            let text = Text::<UPx>::new(&item.text, cushy::kludgine::Color::WHITE);

            if context.gfx.measure_text(text).size.width > w {
                w = context.gfx.measure_text(text).size.width;
            }
        }
        let w = if let ConstraintLimit::Fill(w) = _available_space.width {
            w
        } else {
            w
        };
        Size::new(w, height + padding)
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
            self.modal.dismiss();
            (self.action)(
                &mut context.for_other(&self.owner_id).unwrap(),
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
