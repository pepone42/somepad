use cushy::{
    context::EventContext,
    figures::{
        units::{Px, UPx},
        IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero,
    },
    kludgine::{app::winit::event::MouseButton, shapes::Shape, text::Text, DrawableExt},
    styles::components,
    value::{Destination, Dynamic, Source},
    widget::{Widget, HANDLED, IGNORED},
    ConstraintLimit,
};
use ndoc::Document;

#[derive(Debug)]
pub struct OpenedEditor {
    documents: Dynamic<Vec<Dynamic<Document>>>,
    current_doc: Dynamic<usize>,
    hovered_idx: Dynamic<Option<usize>>,
    //pub width: Dynamic<Px>,
}

impl OpenedEditor {
    pub fn new(documents: Dynamic<Vec<Dynamic<Document>>>, current_doc: Dynamic<usize>) -> Self {
        OpenedEditor {
            documents,
            current_doc,
            hovered_idx: Dynamic::new(None),
        }
    }
}

impl Widget for OpenedEditor {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.gfx.scale())
            .round();
        let padding = Point::new(padding, padding);

        context.redraw_when_changed(&self.hovered_idx);
        let bg_hovered_color = context.get(&components::DefaultActiveBackgroundColor);
        let fg_hovered_color = context.get(&components::DefaultActiveForegroundColor);

        let bg_selected_color = context.get(&components::DefaultHoveredBackgroundColor);
        let fg_selected_color = context.get(&components::DefaultHoveredForegroundColor);
        
        let fg_color = context.get(&components::TextColor);
        let bg_color = context.get(&components::WidgetBackground);

        let scale = context.gfx.scale();
        let size = context.gfx.size();
        let line_height = context.gfx.line_height().into_upx(scale);
        let current_doc = self.current_doc.get();

        context.apply_current_font_settings();

        context.fill(bg_color);
        let mut y = Px::ZERO;
        for (i, doc) in self.documents.get().iter().enumerate() {
            if i == current_doc {
                context.gfx.draw_shape(
                    Shape::filled_rect(
                        Rect::new(
                            Point::new(Px::ZERO, y),
                            Size::new(size.width, line_height).into_signed(),
                        ),
                        bg_selected_color,
                    )
                    .translate_by(padding),
                );
            }

            if let Some(idx) = self.hovered_idx.get() {
                if i == idx {
                    context.gfx.draw_shape(
                        Shape::filled_rect(
                            Rect::new(
                                Point::new(Px::ZERO, y),
                                Size::new(size.width, line_height).into_signed(),
                            ),
                            bg_hovered_color,
                        )
                        .translate_by(padding),
                    );
                }
            }

            let txt_color = match (self.hovered_idx.get(), current_doc) {
                (Some(idx), _) if i == idx => fg_hovered_color,
                (_, idx) if i == idx => fg_selected_color,
                _ => fg_color,
            };

            let text = doc.get().title();

            let text = Text::new(&text, txt_color);
            context
                .gfx
                .draw_text(text.translate_by(padding + Point::new(Px::ZERO, y)));
            y += line_height.into_signed();
        }
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

        let h = UPx::new(self.documents.get().len() as _)
            * context.gfx.line_height().into_upx(context.gfx.scale());

        // TODO, this is very wrong, we should be measuring text here
        // We should also cache the value and/or text layout
        let longest_item = self
            .documents
            .get()
            .iter()
            .map(|d| d.get().title())
            .max_by_key(|s| s.len())
            .unwrap_or_default();
        let text = Text::<UPx>::new(&longest_item, context.get(&components::TextColor));
        let mtext = context.gfx.measure_text(text);

        Size::new(mtext.size.width + padding, h + padding)
    }

    fn hit_test(
        &mut self,
        _location: Point<Px>,
        _context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
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

        let idx = (location.y
            / context
                .kludgine
                .line_height()
                .into_px(context.kludgine.scale()))
        .get() as usize;
        if idx < self.documents.get().len() {
            self.hovered_idx.replace(Some(idx));
        } else {
            self.hovered_idx.replace(None);
        }
        None
    }
    fn unhover(&mut self, _context: &mut EventContext<'_>) {
        self.hovered_idx.replace(None);
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        _button: MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let location = location - padding;
        let idx = (location.y
            / context
                .kludgine
                .line_height()
                .into_px(context.kludgine.scale()))
        .get() as usize;
        if idx < self.documents.get().len() {
            self.current_doc.replace(idx);
            HANDLED
        } else {
            IGNORED
        }
    }
}

#[derive(Debug)]
pub struct ResizeHandle {
    width: Dynamic<Px>,
    hovered: Dynamic<bool>,
    dragged: Dynamic<bool>,
    clip_rect: Rect<UPx>,
}

impl ResizeHandle {
    pub fn new(width: Dynamic<Px>) -> Self {
        ResizeHandle {
            width: width.clone(),
            hovered: Dynamic::new(false),
            dragged: Dynamic::new(false),
            clip_rect: Rect::ZERO,
        }
    }
}

impl Widget for ResizeHandle {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        self.clip_rect = context.gfx.clip_rect();

        context.redraw_when_changed(&self.hovered);
        context.redraw_when_changed(&self.dragged);
        if self.hovered.get() || self.dragged.get() {
            context.fill(context.get(&components::DefaultHoveredBackgroundColor));
        } else {
            context.fill(context.get(&components::WidgetBackground));
        }
    }
    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        _context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        Size::new(UPx::new(5), available_space.height.max())
    }
    fn hover(
        &mut self,
        _location: Point<Px>,
        _context: &mut EventContext<'_>,
    ) -> Option<cushy::kludgine::app::winit::window::CursorIcon> {
        self.hovered.replace(true);
        Some(cushy::kludgine::app::winit::window::CursorIcon::EwResize)
    }
    fn unhover(&mut self, _context: &mut EventContext<'_>) {
        self.hovered.replace(false);
    }
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_>) -> bool {
        true
    }
    fn mouse_down(
        &mut self,
        _location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        _button: cushy::kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        self.dragged.replace(true);
        HANDLED
    }
    fn mouse_up(
        &mut self,
        _location: Option<Point<Px>>,
        _device_id: cushy::window::DeviceId,
        _button: cushy::kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_>,
    ) {
        self.dragged.replace(false);
    }
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        _button: cushy::kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_>,
    ) {
        *self.width.lock() = self.clip_rect.origin.x.into_signed() + location.x;
    }
}
