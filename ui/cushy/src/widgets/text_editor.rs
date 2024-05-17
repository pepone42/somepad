use std::os::raw;

use cushy::kludgine::text::Text;
use cushy::kludgine::wgpu::hal::auxil::db;
use cushy::styles::components::{CornerRadius, SurfaceColor};
use cushy::value::Dynamic;

use cushy::figures::units::{self, Lp, Px, UPx};
use cushy::figures::{Abs, FloatConversion, Fraction, Point, Rect, Round, ScreenScale, Size, Zero};
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};
use cushy::kludgine::cosmic_text::{Attrs, Buffer, Cursor, Family, FontSystem, Metrics};
use cushy::kludgine::shapes::Shape;
use cushy::kludgine::{Drawable, DrawableExt};

use cushy::styles::{Color, CornerRadii, Dimension};
use cushy::value::{Destination, Source};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetTag, WrapperWidget,
    HANDLED, IGNORED,
};

use cushy::{define_components, ModifiersExt};
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
    font_metrics: Metrics,
    font_size: Px,
    line_height: Px,
    scale: Fraction,
}

impl TextEditor {
    pub fn new(doc: Dynamic<ndoc::Document>) -> Self {
        Self {
            doc,
            viewport: Dynamic::new(Rect::default()),
            scroll_controller: Dynamic::new(ScrollController::default()),
            font_metrics: Default::default(),
            font_size: Px::ZERO,
            line_height: Px::ZERO,
            scale: Fraction::ZERO,
        }
    }

    pub fn with_scroller(mut self, scroller: Dynamic<ScrollController>) -> Self {
        self.scroll_controller = scroller;
        self
    }

    fn point_to_grapheme(&self, line: usize, point: Point<Px>) -> usize {
        // TODO: tab support
        let raw_text = self.doc.get().rope.line(line).to_string();
        let mut buffer = Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), self.font_metrics);
        buffer.set_size(
            &mut FONT_SYSTEM.lock().unwrap(),
            10000.,
            self.font_metrics.line_height,
        );
        buffer.set_text(
            &mut FONT_SYSTEM.lock().unwrap(),
            &raw_text,
            Attrs::new().family(Family::Monospace),
            cushy::kludgine::cosmic_text::Shaping::Advanced,
        );
        let byte_idx = buffer
            .hit(point.x.into_float(), point.y.into_float())
            .unwrap_or_default()
            .index;
        rope_utils::byte_to_grapheme(&self.doc.get().rope.line(line as _), byte_idx)
    }

    fn grapheme_to_point(&self, line: usize, index: usize) -> Px {
        // TODO: tab support
        let raw_text = self.doc.get().rope.line(line).to_string();
        let mut buffer = Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), self.font_metrics);
        buffer.set_size(
            &mut FONT_SYSTEM.lock().unwrap(),
            10000.,
            self.font_metrics.line_height,
        );
        buffer.set_text(
            &mut FONT_SYSTEM.lock().unwrap(),
            &raw_text,
            Attrs::new().family(Family::Monospace),
            cushy::kludgine::cosmic_text::Shaping::Advanced,
        );
        let col = rope_utils::grapheme_to_byte(&self.doc.get().rope.line(line), index);
        let c_start = Cursor::new(0, col);
        let c_end = Cursor::new(0, col + 1);
        buffer.line_layout(&mut FONT_SYSTEM.lock().unwrap(), 0);
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
            let main_selection_head_x = self.grapheme_to_point(
                self.doc.get().selections[0].head.line,
                self.doc.get().selections[0].head.column,
            );
            self.scroll_controller.lock().make_region_visible(Rect::new(
                Point::new(
                    Px::ZERO + main_selection_head_x - 10,
                    Px::ZERO
                        + Px::new(self.doc.get().selections[0].head.line as i32) * self.line_height
                        - 10,
                ),
                Size::new(Px::new(35), self.line_height + 20),
            ));
        }
    }
}

impl Widget for TextEditor {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let first_line = (-context.gfx.translation().y / self.line_height) - 1;
        let last_line = first_line
            + (context
                .gfx
                .clip_rect()
                .size
                .height
                .into_px(context.gfx.scale())
                / self.line_height)
            + 1;

        let first_line = first_line.get().max(0) as usize;
        let last_line = last_line.get() as usize;

        context.gfx.set_font_size(Lp::points(12));

        context.fill(Color::new(0x34, 0x3D, 0x46, 0xFF));
        let doc = self.doc.get();

        for i in first_line..last_line {
            let y = units::Px::new(i as _) * self.line_height;
            let slice = doc.rope.slice(..);
            let raw_text = ndoc::rope_utils::get_line_info(&slice, i as _, 4);
            let attrs = Attrs::new().family(Family::Monospace);

            context.gfx.set_text_attributes(attrs);

            if let Some(sl) = doc.get_style_line_info(i as _) {
                let mut buffer = Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), self.font_metrics);
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
                buffer.set_size(
                    &mut FONT_SYSTEM.lock().unwrap(),
                    10000.,
                    self.font_metrics.line_height,
                );

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
            .filter(|s| s.head.line >= first_line && s.head.line <= last_line)
        {
            let head = self.grapheme_to_point(s.head.line, s.head.column).floor();

            context.gfx.draw_shape(
                Shape::filled_rect(
                    Rect::new(
                        Point::new(Px::ZERO, Px::ZERO),
                        Size::new(Px::new(1), self.line_height),
                    ),
                    Color::WHITE,
                )
                .translate_by(Point::new(
                    head,
                    Px::new(s.head.line as i32) * self.line_height,
                )),
            );

            // let main_selection_head_x = self.grapheme_to_point(
            //     s.head.line,
            //     s.head.column,
            // );
            // context.gfx.draw_shape(Shape::stroked_rect(Rect::new(
            //     Point::new(
            //         Px::ZERO + main_selection_head_x - 10,
            //         Px::ZERO
            //             + Px::new(s.head.line as i32) * self.line_height
            //             - 10,
            //     ),
            //     Size::new(Px::new(35), self.line_height + 20),
            // ),Color::WHITE).translate_by(Point::ZERO));
        }
    }

    fn layout(
        &mut self,
        _available_space: Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        if context.gfx.scale() != self.scale {
            self.scale = context.gfx.scale();
            self.line_height = context.get(&LineHeight).into_px(context.gfx.scale()).ceil();
            self.font_size = context.get(&TextSize).into_px(context.gfx.scale()).ceil();
            self.font_metrics =
                Metrics::new(self.font_size.into_float(), self.line_height.into_float());
        }
        let height = self.doc.get().rope.len_lines() as f32 * self.font_metrics.line_height;

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

        let line = ((self.viewport.get().origin.y + location.y) / self.line_height)
            .floor()
            .get();

        let char_idx = self.point_to_grapheme(line as _, Point::new(location.x, 1.into()));
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
                    let linefeed = self.doc.get().file_info.linefeed.to_string();
                    self.doc.lock().insert(&linefeed);
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
    fn full_control_redraw(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct Gutter {
    doc: Dynamic<Document>,
    scroller: Dynamic<ScrollController>,
    font_metrics: Metrics,
    font_size: Px,
    line_height: Px,
    scale: Fraction,
}

impl Gutter {
    pub fn new(doc: Dynamic<Document>, scroller: Dynamic<ScrollController>) -> Self {
        Self {
            doc,
            scroller,
            font_metrics: Metrics::new(15., 15.),
            font_size: Px::ZERO + 15,
            line_height: Px::ZERO + 15,
            scale: Fraction::ZERO + 1,
        }
    }
}

impl Widget for Gutter {

    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let first_line = (-self.scroller.get().scroll().y / self.font_metrics.line_height) - 1;
        let last_line = first_line
            + (context
                .gfx
                .clip_rect()
                .size
                .height
                .into_px(context.gfx.scale())
                / self.font_metrics.line_height)
            + 1;

        let first_line = first_line.get().max(0) as usize;
        let last_line = (last_line.get() as usize).min(self.doc.get().rope.len_lines());

        context
            .gfx
            .set_font_size(Px::new(self.font_metrics.font_size.ceil() as _));

        context.fill(Color::new(0x34, 0x3D, 0x46, 0xFF));

        for i in first_line..last_line {
            let y = self.scroller.get().scroll().y
                + (units::Px::new(i as _) * self.font_metrics.line_height);

            let attrs = Attrs::new().family(Family::Monospace);

            context.gfx.set_text_attributes(attrs);

            context.gfx.draw_text(
                Text::new(&format!(" {} ", i + 1), Color::WHITE)
                    .translate_by(Point::new(Px::ZERO, y)),
            );
        }
    }
    fn layout(
        &mut self,
        available_space: Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        if context.gfx.scale() != self.scale {
            self.scale = context.gfx.scale();
            self.line_height = context.get(&LineHeight).into_px(context.gfx.scale()).ceil();
            self.font_size = context.get(&TextSize).into_px(context.gfx.scale()).ceil();
            self.font_metrics =
                Metrics::new(self.font_size.into_float(), self.line_height.into_float());
        }
        // I don't understand why the +1 is needed. Without it, the gutter is too short by 1pixel vs the text editor
        // But if I add it, the layout/redraw of the gutter/texteditor is called in a loop
        Size::new(UPx::new(50), available_space.height.max()) 
    }
    fn full_control_redraw(&self) -> bool {
        true
    }
}

#[derive(Debug)]
pub struct CodeEditor {
    child: cushy::widget::WidgetRef,
    scroll_id: WidgetId,
}

impl CodeEditor {
    pub fn new(doc: Dynamic<Document>) -> Self {
         let (scroll_tag, scroll_id) = WidgetTag::new();
        let scroller = Dynamic::new(ScrollController::default());
        let child = Gutter::new(doc.clone(), scroller.clone())
            // .expand_vertically()
            // .width(Px::new(50))
            .and(
                MyScroll::new(
                    TextEditor::new(doc.clone()).with_scroller(scroller.clone()),
                    scroller,
                )
                .make_with_tag(scroll_tag) //.contain().background_color(Color::new(0x34, 0x3D, 0x46, 0xFF))
                .expand(),
            )
            .into_columns()
            .gutter(Px::new(1))
            .with(
                &CornerRadius,
                CornerRadii::from(Dimension::Lp(Lp::points(0))),
            );
        Self {
            child: child.widget_ref(),
            scroll_id,
        }
    }
}

impl WrapperWidget for CodeEditor {
    fn child_mut(&mut self) -> &mut cushy::widget::WidgetRef {
        &mut self.child
    }
    fn hit_test(
        &mut self,
        _location: Point<Px>,
        _context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
    }

    fn mouse_wheel(
        &mut self,
        device_id: cushy::window::DeviceId,
        delta: cushy::kludgine::app::winit::event::MouseScrollDelta,
        phase: cushy::kludgine::app::winit::event::TouchPhase,
        context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        context
            .for_other(&self.scroll_id)
            .unwrap()
            .mouse_wheel(device_id, delta, phase);
        IGNORED
    }
}

define_components! {
    CodeEditor {
        TextSize(Lp, "text_size", Lp::points(11))
        LineHeight(Lp, "line_height", Lp::points(13))
    }
}
