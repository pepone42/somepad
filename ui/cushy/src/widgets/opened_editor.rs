use cushy::{
    figures::{
        units::{Lp, Px, UPx}, IntoSigned, Point, Rect, ScreenScale, Size, Zero
    },
    kludgine::{
        app::winit::event::MouseButton, shapes::Shape, text::Text, wgpu::hal::empty::Context,
        DrawableExt,
    },
    styles::components,
    value::{Destination, Dynamic, Source},
    widget::{Widget, HANDLED, IGNORED},
};
use ndoc::Document;

#[derive(Debug)]
pub struct OpenedEditor {
    documents: Dynamic<Vec<Dynamic<Document>>>,
    current_doc: Dynamic<usize>,
    width: Dynamic<UPx>,
    hovered: Dynamic<bool>,
    dragged: Dynamic<bool>,
}

impl OpenedEditor {
    pub fn new(documents: Dynamic<Vec<Dynamic<Document>>>, current_doc: Dynamic<usize>) -> Self {
        OpenedEditor {
            documents,
            current_doc,
            width: Dynamic::new(UPx::new(100)),
            hovered: Dynamic::new(false),
            dragged: Dynamic::new(false),
        }
    }

    fn on_resize_handle(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        location.x > self.width.get().into_px(context.kludgine.scale()) - 5
    }
}

impl Widget for OpenedEditor {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let bg_hovered_color = context.get(&components::DefaultHoveredBackgroundColor);
        let bg_selected_color = context.get(&components::DefaultActiveBackgroundColor);
        let fg_selected_color = context.get(&components::DefaultActiveForegroundColor);
        let fg_hovered_color = context.get(&components::DefaultHoveredForegroundColor);
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
                        Rect::new(Point::new(Px::ZERO, y), Size::new(size.width, line_height).into_signed()),
                        bg_selected_color,
                    )
                    .translate_by(Point::ZERO),
                );
            }

            let mut text = if let Some(file_name) = doc.get().file_name {
                file_name.file_name().unwrap().to_string_lossy().to_string()
            } else {
                format!("Untitled {}", doc.get().id())
            };
            if i == self.current_doc.get() {
                text.push_str(" (current)");
            }
            let text = Text::new(&text, if i == current_doc {fg_selected_color} else { fg_color });
            context
                .gfx
                .draw_text(text.translate_by(Point::new(Px::ZERO, y)));
            y += line_height.into_signed();
        }

        if self.hovered.get() || self.dragged.get() {
            let width = self.width.get().into_px(context.gfx.scale());
            let scale = context.gfx.scale();
            let height = context.gfx.size().height.into_px(scale);
            context.gfx.draw_shape(
                Shape::filled_rect(
                    Rect::new(
                        Point::new(width - 5, Px::ZERO),
                        Size::new(Px::new(5), height),
                    ),
                    bg_hovered_color,
                )
                .translate_by(Point::ZERO),
            );
        }
    }

    fn layout(
        &mut self,
        _available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<cushy::figures::units::UPx> {
        let h = UPx::new(self.documents.get().len() as _)
            * context.gfx.line_height().into_upx(context.gfx.scale());
        Size::new(self.width.get(), h)
    }

    fn hit_test(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true //self.on_resize_handle(location, context)
    }

    fn hover(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> Option<cushy::kludgine::app::winit::window::CursorIcon> {
        context.redraw_when_changed(&self.hovered);
        if self.on_resize_handle(location, context) {
            self.hovered.replace(true);
            Some(cushy::kludgine::app::winit::window::CursorIcon::EwResize)
        } else {
            self.hovered.replace(false);
            None
        }
    }
    fn unhover(&mut self, context: &mut cushy::context::EventContext<'_>) {
        context.redraw_when_changed(&self.hovered);
        self.hovered.replace(false);
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        button: MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        if button == MouseButton::Left && self.on_resize_handle(location, context) {
            HANDLED
        } else {
            IGNORED
        }
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: cushy::window::DeviceId,
        button: MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) {
        if button == MouseButton::Left {
            context.invalidate_when_changed(&self.dragged);
            *self.dragged.lock() = false;
        }
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) {
        if button == MouseButton::Left {
            context.invalidate_when_changed(&self.width);
            *self.dragged.lock() = true;
            *self.width.lock() = location.x.into_upx(context.kludgine.scale());
        } else {
            *self.dragged.lock() = false;
        }
    }
}
