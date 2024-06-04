use cushy::{
    figures::{
        units::{Px, UPx},
        Point, ScreenScale, Size, Zero,
    },
    kludgine::{
        app::winit::event::MouseButton,
        text::Text,
        DrawableExt,
    },
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
}

impl OpenedEditor {
    pub fn new(documents: Dynamic<Vec<Dynamic<Document>>>, current_doc: Dynamic<usize>) -> Self {
        OpenedEditor {
            documents,
            current_doc,
            width: Dynamic::new(UPx::new(100)),
            hovered: Dynamic::new(false),
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
        context.apply_current_font_settings();
        let mut y = Px::ZERO;
        for (i, doc) in self.documents.get().iter().enumerate() {
            let mut text = if let Some(file_name) = doc.get().file_name {
                file_name.file_name().unwrap().to_string_lossy().to_string()
            } else {
                format!("Untitled {}", i)
            };
            if i == self.current_doc.get() {
                text.push_str(" (current)");
            }
            let text = Text::new(&text, cushy::kludgine::Color::WHITE);
            context
                .gfx
                .draw_text(text.translate_by(Point::new(Px::ZERO, y)));
            y += 20;
        }
        // TODO: handle is always visible even if not hovered
        // if self.hovered.get() {
        //     let width = self.width.get().into_px(context.gfx.scale());
        //     let scale = context.gfx.scale();
        //     let height = context.inner_size().get().height.into_px(scale);
        //     context.gfx.draw_shape(
        //         Shape::filled_rect(
        //             Rect::new(
        //                 Point::new(width - 5, Px::ZERO),
        //                 Size::new(
        //                     Px::new(5),
        //                     height,
        //                 ),
        //             ),
        //             cushy::kludgine::Color::WHITE,
        //         )
        //         .translate_by(Point::ZERO),
        //     );
        // }
    }

    fn layout(
        &mut self,
        available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<cushy::figures::units::UPx> {
        let h = UPx::new(self.documents.get().len() as _) * 20;
        Size::new(self.width.get(), h)
    }

    fn hit_test(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        self.on_resize_handle(location, context)
    }

    fn hover(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> Option<cushy::kludgine::app::winit::window::CursorIcon> {
        context.redraw_when_changed(&self.hovered);
        if self.on_resize_handle(location, context) {
            dbg!("resize", location, self.width.get());
            self.hovered.replace(true);
            Some(cushy::kludgine::app::winit::window::CursorIcon::EwResize)
        } else {
            self.hovered.replace(false);
            None
        }
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: cushy::window::DeviceId,
        button: MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        if button == MouseButton::Left && self.on_resize_handle(location, context) {
            HANDLED
        } else {
            IGNORED
        }
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: cushy::window::DeviceId,
        button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) {
        if button == MouseButton::Left {
            context.invalidate_when_changed(&self.width);
            *self.width.lock() = location.x.into_upx(context.kludgine.scale());
        }
    }
}
