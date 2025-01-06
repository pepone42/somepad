use cushy::{
    figures::{units::Px, FloatConversion, Point, Rect, ScreenScale, Size, Zero},
    kludgine::{
        app::winit::keyboard::{Key, NamedKey},
        cosmic_text::{rustybuzz::shape, Attrs, Buffer, Edit, Editor, Metrics, Shaping},
        image::buffer,
        shapes::Shape,
        Drawable, DrawableExt, Kludgine,
    },
    styles::components,
    value::{Dynamic, Source},
    widget::{Widget, HANDLED, IGNORED},
};

#[derive(Debug)]
pub struct Input {
    text: Dynamic<String>,
    cursor: cushy::kludgine::cosmic_text::Cursor,
    editor: Editor,
}

impl Input {
    pub fn new(text: Dynamic<String>) -> Self {
        let buffer = Buffer::new_empty(Metrics::new(0.0, 0.0));
        Input {
            text,
            cursor: cushy::kludgine::cosmic_text::Cursor::new(1, 4),
            editor: Editor::new(buffer),
        }
    }
}

impl Widget for Input {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let scale = context.gfx.scale();
        let base_text_size = context
            .get(&components::BaseTextSize)
            .into_px(scale)
            .into_float();
        let line_height = context.gfx.line_height().into_upx(scale).into_float();
        let size = context.gfx.size();
        let color_fg = context.get(&components::TextColor);

        //let font_system = context.gfx.font_system();
        let metrics = Metrics::new(base_text_size, line_height);
        self.editor.with_buffer(|buffer| {
            buffer.set_metrics(context.gfx.font_system(), metrics);
            buffer.set_text(
                context.gfx.font_system(),
                &self.text.get(),
                Attrs::new(),
                Shaping::Advanced,
            );

            // self.buffer = Buffer::new(context.gfx.font_system(), metrics);
            // self.buffer.set_text(
            //     context.gfx.font_system(),
            //     &self.text.get(),
            //     Attrs::new(),
            //     Shaping::Advanced,
            // );
            // self.buffer.set_size(
            //     context.gfx.font_system(),
            //     Some(size.width.into()),
            //     Some(size.height.into()),
            // );
            // context.gfx.draw_text_buffer(
            //     Drawable {
            //         source: &self.buffer,
            //         translation: Point::<Px>::default(),
            //         opacity: None,
            //         rotation: None,
            //         scale: None,
            //     },
            //     color_fg,
            //     cushy::kludgine::text::TextOrigin::TopLeft,
            // );
        });
        self.editor.draw(font_system, cache, text_color, cursor_color, selection_color, selected_text_color, f);
        // let c = self
        //     .buffer
        //     .layout_cursor(context.gfx.font_system(), self.cursor);

        // for lr in self.buffer.layout_runs() {
        //     dbg!(lr.line_top);
        //     dbg!(lr.glyphs[0].x_offset);
        // }

        // if let Some(c) = c {
        //     let line = &self.buffer.lines[c.line];
        //     let linetop = self
        //         .buffer
        //         .layout_runs()
        //         .filter(|lr| lr.line_i == c.line)
        //         .map(|lr| (lr.line_top, lr.line_height))
        //         .collect::<Vec<_>>();
        //     if let Some(ref layout) = line.layout_opt() {
        //         let glyph = &layout[c.layout].glyphs[c.glyph];

        //         dbg!(
        //             glyph,
        //             &layout[c.layout].max_ascent,
        //             &layout[c.layout].max_descent,
        //             c.line as f32 * line_height
        //         );
        //         context.gfx.draw_shape(
        //             Shape::filled_rect(
        //                 Rect::new(
        //                     Point::new(glyph.x.into(), linetop[c.layout].0.into()),
        //                     Size::new(Px::new(1), (linetop[c.layout].1).into()),
        //                 ),
        //                 color_fg,
        //             )
        //             .translate_by(Point::ZERO),
        //         );
        //     }
        //     dbg!(c);
        // }
    }

    fn keyboard_input(
        &mut self,
        device_id: cushy::window::DeviceId,
        input: cushy::window::KeyEvent,
        is_synthetic: bool,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        match input.logical_key {
            Key::Named(NamedKey::ArrowLeft) => HANDLED,
            _ => IGNORED,
        }
    }

    fn focus(&mut self, context: &mut cushy::context::EventContext<'_>) {
        dbg!("focus");
    }

    fn accept_focus(&mut self, context: &mut cushy::context::EventContext<'_>) -> bool {
        true
    }

    fn hit_test(
        &mut self,
        location: Point<Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
    }
    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: cushy::window::DeviceId,
        button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        context.focus();
        HANDLED
    }
}
