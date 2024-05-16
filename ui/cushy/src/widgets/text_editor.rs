use cushy::kludgine::text::Text;
use cushy::styles::components::{CornerRadius, SurfaceColor};
use cushy::value::Dynamic;

use cushy::figures::units::{self, Lp, Px, UPx};
use cushy::figures::{Abs, FloatConversion, Point, Rect, Round, ScreenScale, Size, Zero};
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};
use cushy::kludgine::cosmic_text::{Attrs, Buffer, Cursor, Family, FontSystem, Metrics};
use cushy::kludgine::shapes::Shape;
use cushy::kludgine::{Drawable, DrawableExt};

use cushy::styles::{Color, CornerRadii, Dimension};
use cushy::value::{Destination, Source};
use cushy::widget::{EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetTag, WrapperWidget, HANDLED, IGNORED};

use cushy::ModifiersExt;
use ndoc::{rope_utils, Document};
use scroll::ScrollController;

use crate::shortcut::event_match;
use crate::{FONT_SYSTEM, VIEW_SHORTCUT};

use super::scroll::{self, MyScroll};

#[derive(Debug)]
pub struct TextEditor {
    pub doc: Dynamic<ndoc::Document>,
    viewport: Dynamic<Rect<Px>>,
    scroll_controller: Dynamic<ScrollController>,
}

impl TextEditor {
    pub fn new(doc: Dynamic<ndoc::Document>) -> Self {
        Self {
            doc,
            viewport: Dynamic::new(Rect::default()),
            scroll_controller: Dynamic::new(ScrollController::default()),
        }
    }

    pub fn with_scroller(mut self, scroller: Dynamic<ScrollController>) -> Self {
        self.scroll_controller = scroller;
        self
    }

    fn point_to_grapheme(
        doc: &ndoc::Document,
        line: usize,
        point: Point<Px>,
        font_system: &mut FontSystem,
    ) -> usize {
        // TODO: tab support
        let raw_text = doc.rope.line(line).to_string();
        let mut buffer = Buffer::new(font_system, Metrics::new(15.0, 20.0));
        buffer.set_size(font_system, 10000., 20.);
        buffer.set_text(
            font_system,
            &raw_text,
            Attrs::new().family(Family::Monospace),
            cushy::kludgine::cosmic_text::Shaping::Advanced,
        );
        let byte_idx = buffer
            .hit(point.x.into_float(), point.y.into_float())
            .unwrap_or_default()
            .index;
        rope_utils::byte_to_grapheme(&doc.rope.line(line as _), byte_idx)
    }

    fn grapheme_to_point(
        doc: &ndoc::Document,
        line: usize,
        index: usize,
        font_system: &mut FontSystem,
    ) -> Px {
        // TODO: tab support
        let raw_text = doc.rope.line(line).to_string();
        let mut buffer = Buffer::new(font_system, Metrics::new(15.0, 20.0));
        buffer.set_size(font_system, 1000., 1000.);
        buffer.set_text(
            font_system,
            &raw_text,
            Attrs::new().family(Family::Monospace),
            cushy::kludgine::cosmic_text::Shaping::Advanced,
        );
        let col = rope_utils::grapheme_to_byte(&doc.rope.line(line), index);
        let c_start = Cursor::new(0, col);
        let c_end = Cursor::new(0, col + 1);
        buffer.line_layout(font_system, 0);
        buffer
            .layout_runs()
            .nth(0)
            .unwrap()
            .highlight(c_start, c_end)
            .unwrap_or_default()
            .0
            .into()
    }

    pub fn refocus_main_selection(&self) {
        if self.doc.get().selections.len() == 1 {
            let main_selection_head_x = TextEditor::grapheme_to_point(
                &self.doc.get(),
                self.doc.get().selections[0].head.line,
                self.doc.get().selections[0].head.column,
                &mut FONT_SYSTEM.lock().unwrap(),
            );
            self.scroll_controller.lock().make_region_visible(Rect::new(
                Point::new(
                    Px::ZERO + main_selection_head_x - 10,
                    Px::ZERO + Px::new(self.doc.get().selections[0].head.line as i32 * 20) - 10,
                ),
                Size::new(Px::new(35), Px::new(40)),
            ));
        }
    }
}

impl Widget for TextEditor {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let first_line = -context.gfx.translation().y / 20.0;
        let last_line = first_line
            + (context
                .gfx
                .clip_rect()
                .size
                .height
                .into_px(context.gfx.scale())
                / 20.0);

        let first_line = first_line.get() as usize;
        let last_line = last_line.get() as usize;

        context.gfx.set_font_size(Lp::points(12));

        context.fill(Color::new(0x34, 0x3D, 0x46, 0xFF));
        let doc = self.doc.get_tracking_redraw(context);

        for i in first_line..last_line {
            let y = units::Px::new(i as _) * 20;
            let slice = doc.rope.slice(..);
            let raw_text = ndoc::rope_utils::get_line_info(&slice, i as _, 4);
            let attrs = Attrs::new().family(Family::Monospace);

            context.gfx.set_text_attributes(attrs);

            if let Some(sl) = doc.get_style_line_info(i as _) {
                let mut buffer =
                    Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), Metrics::new(15.0, 20.0));
                let mut spans = Vec::new();
                for s in sl.iter() {
                    let t = &raw_text[s.range.start..s.range.end];

                    let col = cushy::kludgine::cosmic_text::Color::rgba(
                        s.style.foreground.r,
                        s.style.foreground.g,
                        s.style.foreground.b,
                        s.style.foreground.a,
                    );

                    spans.push((t, attrs.color(col)));
                }
                buffer.set_rich_text(
                    &mut FONT_SYSTEM.lock().unwrap(),
                    spans,
                    attrs,
                    cushy::kludgine::cosmic_text::Shaping::Advanced,
                );
                buffer.set_size(&mut FONT_SYSTEM.lock().unwrap(), 10000., 20.);

                context.gfx.draw_text_buffer(
                    Drawable {
                        source: &buffer,
                        translation: Point::<Px>::default(),
                        opacity: None,
                        rotation: None,
                        scale: None,
                    }
                    .translate_by(Point::new(Px::ZERO, y)),
                    Color::WHITE,
                    cushy::kludgine::text::TextOrigin::TopLeft,
                );
            }
        }

        // draw cursors
        for s in doc
            .selections
            .iter()
            .filter(|s| s.head.line >= first_line && s.head.line < last_line)
        {
            let head = TextEditor::grapheme_to_point(
                &doc,
                s.head.line,
                s.head.column,
                &mut FONT_SYSTEM.lock().unwrap(),
            )
            .floor();

            context.gfx.draw_shape(
                Shape::filled_rect(
                    Rect::new(
                        Point::new(Px::ZERO, Px::ZERO),
                        Size::new(Px::new(1), Px::new(20)),
                    ),
                    Color::WHITE,
                )
                .translate_by(Point::new(head, Px::new(s.head.line as i32 * 20))),
            );
        }
    }

    fn layout(
        &mut self,
        _available_space: Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let height = self.doc.get().rope.len_lines() as f32 * 20.0;

        self.viewport.set(Rect::new(
            context.gfx.translation().abs(),
            context.gfx.size().into_px(context.gfx.scale()),
        ));

        Size::new(UPx::new(10000), UPx::new(height.ceil() as _))
    }

    fn accept_focus(&mut self, _context: &mut cushy::context::EventContext<'_>) -> bool {
        true
    }

    fn hit_test(
        &mut self,
        _location: Point<units::Px>,
        _context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        location: Point<units::Px>,
        _device_id: cushy::window::DeviceId,
        _button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        if !context.enabled() {
            return IGNORED;
        }
        context.focus();

        let line = ((self.viewport.get().origin.y + location.y) / 20)
            .floor()
            .get();

        let char_idx = TextEditor::point_to_grapheme(
            &self.doc.get(),
            line as _,
            Point::new(location.x, 1.into()),
            context.kludgine.font_system(),
        );
        let col = rope_utils::byte_to_grapheme(&self.doc.get().rope.line(line as _), char_idx);
        dbg!(line, char_idx, col);

        IGNORED
    }

    fn keyboard_input(
        &mut self,
        _device_id: cushy::window::DeviceId,
        input: cushy::window::KeyEvent,
        _is_synthetic: bool,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        if !context.enabled() {
            return IGNORED;
        }

        if input.state == ElementState::Pressed && context.modifiers().possible_shortcut() {
            let v = VIEW_SHORTCUT.lock().unwrap();
            let id = context.widget.widget().id();
            for (shortcut, cmd) in v.iter() {
                if event_match(&input, context.modifiers(), shortcut.clone()) {
                    (cmd.action)(id, self);
                    return HANDLED;
                }
            }
        }

        if input.state == ElementState::Pressed && matches!(input.logical_key, Key::Named(_)) {
            match input.logical_key {
                Key::Named(NamedKey::Backspace) => {
                    self.doc.lock().backspace();
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::Delete) => {
                    self.doc.lock().delete();
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowLeft) if context.modifiers().word_select() => {
                    self.doc.lock().move_selections_word(
                        ndoc::MoveDirection::Left,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowRight) if context.modifiers().word_select() => {
                    self.doc.lock().move_selections_word(
                        ndoc::MoveDirection::Right,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    self.doc.lock().move_selections(
                        ndoc::MoveDirection::Left,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.doc.lock().move_selections(
                        ndoc::MoveDirection::Right,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.doc
                        .lock()
                        .move_selections(ndoc::MoveDirection::Up, context.modifiers().only_shift());
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.doc.lock().move_selections(
                        ndoc::MoveDirection::Down,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection();
                    return HANDLED;
                }
                Key::Named(NamedKey::Enter) => {
                    let mut doc = self.doc.lock();
                    let linefeed = doc.file_info.linefeed.to_string();
                    doc.insert(&linefeed);
                    self.refocus_main_selection();
                    return HANDLED;
                }
                _ => {}
            }
        }

        match input.text {
            Some(t) if !context.modifiers().possible_shortcut() => {
                self.doc.lock().insert(&t);
                self.refocus_main_selection();

                HANDLED
            }
            _ => IGNORED,
        }
    }
}

#[derive(Debug)]
pub struct Gutter {
    doc: Dynamic<Document>,
    scroller: Dynamic<ScrollController>,
}

impl Gutter {
    pub fn new(doc: Dynamic<Document>, scroller: Dynamic<ScrollController>) -> Self {
        Self { doc, scroller}
    }
}

impl Widget for Gutter {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let first_line = -self.scroller.get().scroll().y / 20.0;
        let last_line = first_line
            + (context
                .gfx
                .clip_rect()
                .size
                .height
                .into_px(context.gfx.scale())
                / 20.0);

        let first_line = first_line.get() as usize;
        let last_line = last_line.get() as usize;

        context.gfx.set_font_size(Lp::points(12));

        context.fill(Color::new(0x34, 0x3D, 0x46, 0xFF));
        let doc = self.doc.get_tracking_redraw(context);

        for i in first_line..last_line {
            let y = self.scroller.get().scroll().y + (units::Px::new(i as _) * 20);
            let slice = doc.rope.slice(..);
            let raw_text = ndoc::rope_utils::get_line_info(&slice, i as _, 4);
            let attrs = Attrs::new().family(Family::Monospace);

            context.gfx.set_text_attributes(attrs);

            context.gfx.draw_text(
                Text::new(&format!("{}", i), Color::WHITE).translate_by(Point::new(Px::ZERO, y)),
            );
        }
    }
}

#[derive(Debug)]
pub struct CodeEditor {
    doc: Dynamic<Document>,
    child: cushy::widget::WidgetRef,
    scroll_id: WidgetId,
}

impl CodeEditor {
    pub fn new(doc: Dynamic<Document>) -> Self {
        let (scroll_tag,scroll_id) = WidgetTag::new();
        let scroller = Dynamic::new(ScrollController::default());
        let child = Gutter::new(doc.clone(), scroller.clone())
            .expand_vertically()
            .width(Px::new(50)).contain().background_color(Color::new(0x34, 0x3D, 0x46, 0xFF))
            .and(
                MyScroll::new(
                    TextEditor::new(doc.clone())
                        .with_scroller(scroller.clone())
                        ,
                    scroller,
                ).make_with_tag(scroll_tag).contain().background_color(Color::new(0x34, 0x3D, 0x46, 0xFF))
                .expand(),
            )
            .into_columns().gutter(Px::new(1)).with(
                &CornerRadius,
                CornerRadii::from(Dimension::Lp(Lp::points(0))),
            );
        Self {
            doc,
            child: child.widget_ref(),
            scroll_id
        }
    }
}

impl WrapperWidget for CodeEditor {
    fn child_mut(&mut self) -> &mut cushy::widget::WidgetRef {
        &mut self.child
    }
    fn hit_test(&mut self, location: Point<Px>, context: &mut cushy::context::EventContext<'_>) -> bool {
        true
    }

    fn mouse_wheel(
            &mut self,
            device_id: cushy::window::DeviceId,
            delta: cushy::kludgine::app::winit::event::MouseScrollDelta,
            phase: cushy::kludgine::app::winit::event::TouchPhase,
            context: &mut cushy::context::EventContext<'_>,
        ) -> EventHandling {
            context.for_other(&self.scroll_id).unwrap().mouse_wheel(device_id, delta, phase);
            IGNORED
    }
}
