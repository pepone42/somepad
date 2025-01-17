use cushy::{
    figures::{units::Px, FloatConversion, Point, Rect, ScreenScale, Size, Unit, Zero},
    kludgine::{
        app::winit::{
            event::ElementState,
            keyboard::{Key, NamedKey},
        },
        cosmic_text::{rustybuzz::shape, Attrs, Buffer, Cursor, Edit, Editor, Metrics, Shaping},
        image::buffer,
        shapes::Shape,
        Drawable, DrawableExt, Kludgine,
    },
    styles::components,
    value::{Destination, Dynamic, Source},
    widget::{Widget, HANDLED, IGNORED},
};

use crate::shortcut::ModifiersCustomExt;

#[derive(Debug)]
pub struct Input<'buffer> {
    text: Dynamic<String>,
    redraw: Dynamic<bool>,
    editor: Editor<'buffer>,
}

impl<'buffer> Input<'buffer> {
    pub fn new(text: Dynamic<String>) -> Self {
        let buffer = Buffer::new_empty(Metrics::new(9.0, 9.0));

        let mut editor = Editor::new(buffer);
        editor.set_cursor(Cursor::new(0, 0));
        Input {
            text,
            redraw: Dynamic::new(false),
            editor,
        }
    }
}

impl<'buffer: 'static> Widget for Input<'buffer> {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        context.redraw_when_changed(&self.redraw);

        if context.focused(true) {
            context.draw_focus_ring();
        }

        let scale = context.gfx.scale();
        let base_text_size = context
            .get(&components::BaseTextSize)
            .into_px(scale)
            .into_float();
        let line_height = context.gfx.line_height().into_upx(scale).into_float();
        let size = context.gfx.size();
        let color_fg = context.get(&components::TextColor);
        let color_cursor_fg = context.get(&components::HighlightColor);

        //let font_system = context.gfx.font_system();
        let metrics = Metrics::new(base_text_size, line_height);
        let cursor = self.editor.cursor();
        //dbg!(self.editor.cursor_position(), self.editor.selection());

        let cursor_prosition = self.editor.cursor_position();
        let selection = self.editor.selection_bounds();

        self.editor.with_buffer_mut(|buffer| {
            // draw selection
            if let Some(selection) = selection {
                for run in buffer.layout_runs() {
                    let h = run.highlight(selection.0, selection.1);
                    dbg!(h);
                    if let Some(h) = h {
                        let line_height = run.line_height.ceil() as i32;
                        let start = Px::new(h.0.ceil() as i32);
                        let end = Px::new(h.1.ceil() as i32);
                        context.gfx.draw_shape(
                            Shape::filled_rect(
                                Rect::new(
                                    Point::new(start, Px::new(run.line_top.ceil() as _)),
                                    Size::new(end, Px::new(line_height)),
                                ),
                                color_cursor_fg,
                            )
                            .translate_by(Point::ZERO),
                        )
                    }
                }
            }

            // draw text
            buffer.set_metrics(context.gfx.font_system(), metrics);
            buffer.set_text(
                context.gfx.font_system(),
                &self.text.get(),
                Attrs::new(),
                Shaping::Advanced,
            );

            buffer.set_size(
                context.gfx.font_system(),
                Some(size.width.into()),
                Some(size.height.into()),
            );
            context.gfx.draw_text_buffer(
                Drawable::<&Buffer, Px> {
                    source: buffer,
                    translation: Point::default(),
                    opacity: None,
                    rotation: None,
                    scale: None,
                },
                color_fg,
                cushy::kludgine::text::TextOrigin::TopLeft,
            );

            // draw cursor
            let line_height = buffer
                .layout_runs()
                .filter(|lr| lr.line_i == cursor.line)
                .map(|lr| lr.line_height.ceil() as i32)
                .max()
                .unwrap_or(0);
            if let Some(cp) = cursor_prosition {
                context.gfx.draw_shape(
                    Shape::filled_rect(
                        Rect::new(
                            Point::new(Px::new(cp.0), Px::new(cp.1)),
                            Size::new(Px::new(1), Px::new(line_height)),
                        ),
                        color_cursor_fg,
                    )
                    .translate_by(Point::ZERO),
                )
            }
        });
    }

    fn keyboard_input(
        &mut self,
        device_id: cushy::window::DeviceId,
        input: cushy::window::KeyEvent,
        is_synthetic: bool,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        if input.state.is_pressed() {
            if input.modifiers.shift()
                && matches!(
                    self.editor.selection(),
                    cushy::kludgine::cosmic_text::Selection::None
                )
            {
                self.editor
                    .set_selection(cushy::kludgine::cosmic_text::Selection::Normal(
                        self.editor.cursor(),
                    ));
            }
            if !input.modifiers.shift() {
                self.editor
                    .set_selection(cushy::kludgine::cosmic_text::Selection::None);
            }

            match input.logical_key {
                Key::Named(NamedKey::ArrowLeft) => {
                    self.editor.action(
                        context.kludgine.font_system(),
                        cushy::kludgine::cosmic_text::Action::Motion(
                            cushy::kludgine::cosmic_text::Motion::Previous,
                        ),
                    );
                    dbg!("left");
                    self.redraw.toggle();
                    HANDLED
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.editor.action(
                        context.kludgine.font_system(),
                        cushy::kludgine::cosmic_text::Action::Motion(
                            cushy::kludgine::cosmic_text::Motion::Next,
                        ),
                    );
                    dbg!("right");
                    self.redraw.toggle();
                    HANDLED
                }
                _ => IGNORED,
            }
        } else {
            IGNORED
        }
    }

    fn focus(&mut self, context: &mut cushy::context::EventContext<'_>) {
        self.redraw.toggle();
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
