use std::collections::HashMap;
use std::os::raw;

use cushy::kludgine::text::Text;
use cushy::kludgine::wgpu::hal::auxil::db;
use cushy::styles::components::{CornerRadius, SurfaceColor};
use cushy::value::Dynamic;

use cushy::figures::units::{self, Lp, Px, UPx};
use cushy::figures::{
    fraction, Abs, FloatConversion, Fraction, Point, Rect, Round, ScreenScale, Size, Zero,
};
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};
use cushy::kludgine::cosmic_text::{Attrs, Buffer, Cursor, Family, FontSystem, Metrics};
use cushy::kludgine::shapes::{Path, PathBuilder, Shape};
use cushy::kludgine::{Drawable, DrawableExt};

use cushy::styles::{Color, CornerRadii, Dimension};
use cushy::value::{Destination, Source};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetTag, WrapperWidget,
    HANDLED, IGNORED,
};

use cushy::{define_components, ModifiersExt};
use ndoc::{rope_utils, Document, Selection};
use scroll::ScrollController;

use crate::shortcut::event_match;
use crate::{CommandsRegistry, FONT_SYSTEM};

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
    cmd_reg: Dynamic<CommandsRegistry>,
}

impl TextEditor {
    pub fn new(doc: Dynamic<ndoc::Document>, cmd_reg: Dynamic<CommandsRegistry>) -> Self {
        Self {
            doc,
            viewport: Dynamic::new(Rect::default()),
            scroll_controller: Dynamic::new(ScrollController::default()),
            font_metrics: Default::default(),
            font_size: Px::ZERO,
            line_height: Px::ZERO,
            scale: Fraction::ZERO,
            cmd_reg,
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

    fn layout_line(&self, line_idx: usize) -> Option<Buffer> {
        let raw_text =
            ndoc::rope_utils::get_line_info(&self.doc.get().rope.slice(..), line_idx as _, 4)
                .to_string();
        let attrs = Attrs::new().family(Family::Monospace);

        //context.gfx.set_text_attributes(attrs);

        if let Some(sl) = self.doc.get().get_style_line_info(line_idx as _) {
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
            Some(buffer)
        } else {
            None
        }
    }

    fn get_selection_shape(
        &self,
        selection: Selection,
        layouts: &HashMap<usize, Option<Buffer>>,
    ) -> Option<Path<Px, false>> {
        let rope = &self.doc.get().rope;

        let rects = selection
            .areas(rope)
            .iter()
            .filter_map(|a| (layouts.contains_key(&a.line).then_some(*a)))
            .map(|a| {
                // TODO: Use the buffers instead of recalculating it
                let start = self.grapheme_to_point(a.line, a.col_start);
                let end = self.grapheme_to_point(a.line, a.col_end);
                let y = self.line_height * Px::new(a.line as i32);

                Rect::new(
                    Point::new(start.ceil(), y.ceil()),
                    Size::new(
                        end.ceil()
                            + if a.include_eol {
                                //self.char_base_width.ceil()
                                Px::new(15) // TODO, calculate grapheme width
                            } else {
                                Px::ZERO
                            }
                            - start.ceil(),
                        self.line_height.ceil(),
                    ),
                )
            })
            .collect::<Vec<Rect<Px>>>();

        make_selection_path(&rects)
    }

    fn get_selections_shapes(
        &self,
        layouts: &HashMap<usize, Option<Buffer>>,
    ) -> Vec<Path<Px, false>> {
        self.doc
            .get()
            .selections
            .iter()
            .filter_map(|s| self.get_selection_shape(*s, layouts))
            .collect()
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
        let total_line = last_line - first_line;

        context.gfx.set_font_size(Lp::points(12));

        context.fill(Color::new(0x34, 0x3D, 0x46, 0xFF));
        let doc = self.doc.get();

        let buffers = self
            .doc
            .get()
            .rope
            .lines()
            .enumerate()
            .skip(first_line)
            .take(total_line)
            .map(|(i, _)| (i, self.layout_line(i)))
            .collect::<HashMap<usize, Option<Buffer>>>();

        let selections = self
            .doc
            .get()
            .selections
            .iter()
            .map(|s| (s.head.line, s.head.column))
            .collect::<HashMap<usize, usize>>();

        // draw selections
        for path in self.get_selections_shapes(&buffers) {
            // TODO: use correct color
            context
                .gfx
                .draw_shape(path.fill(Color::ALICEBLUE).translate_by(Point::ZERO));
            context
                .gfx
                .draw_shape(path.stroke(Color::BLUE).translate_by(Point::ZERO));
        }

        for i in first_line..last_line {
            let y = units::Px::new(i as _) * self.line_height;
            if let Some(b) = buffers.get(&i) {
                if let Some(b) = b {
                    context.gfx.draw_text_buffer(
                        Drawable {
                            source: b,
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
            let v = self.cmd_reg.get().view_shortcut;
            let id = context.widget.widget().id();
            for (shortcut, cmd) in v.iter() {
                if event_match(&input, context.modifiers(), shortcut.clone()) {
                    (cmd.action)(id, self, context);
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
            font_size: Px::ZERO,
            line_height: Px::ZERO,
            scale: Fraction::ZERO,
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
    pub fn new(doc: Dynamic<Document>, cmd_reg: Dynamic<CommandsRegistry>) -> Self {
        let (scroll_tag, scroll_id) = WidgetTag::new();
        let scroller = Dynamic::new(ScrollController::default());
        let child = Gutter::new(doc.clone(), scroller.clone())
            // .expand_vertically()
            // .width(Px::new(50))
            .and(
                MyScroll::new(
                    TextEditor::new(doc.clone(), cmd_reg).with_scroller(scroller.clone()),
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

fn make_selection_path(rects: &[Rect<Px>]) -> Option<Path<Px, false>> {
    let bevel = Px::new(3);
    let epsilon: f32 = 0.0001;

    let mut left = Vec::with_capacity(rects.len() * 2);
    let mut right = Vec::with_capacity(rects.len() * 2);
    for r in rects
        .iter()
        .filter(|r| ((r.origin.x + r.size.width) - r.origin.x).into_float() > epsilon)
    {
        let x0 = r.origin.x;
        let y0 = r.origin.y;
        let x1 = r.origin.x + r.size.width;
        let y1 = r.origin.y + r.size.height;
        right.push(Point::new(x1, y0));
        right.push(Point::new(x1, y1));
        left.push(Point::new(x0, y0));
        left.push(Point::new(x0, y1));
    }
    left.reverse();

    let points = [right, left].concat();

    #[derive(Clone, Copy, Debug)]
    enum PathEl {
        MoveTo(Point<Px>),
        LineTo(Point<Px>),
        QuadTo(Point<Px>, Point<Px>),
        Close,
    }
    let mut path = Vec::new();

    for i in 0..points.len() {
        let p1 = if i == 0 {
            points[points.len() - 1]
        } else {
            points[i - 1]
        };
        let p2 = points[i];
        let p3 = if i == points.len() - 1 {
            points[0]
        } else {
            points[i + 1]
        };

        let v1 = p2 - p1;
        let v2 = p2 - p3;

        fn cross(v1: Point<Px>, v2: Point<Px>) -> f32 {
            (v1.x * v2.y - v1.y * v2.x).into_float()
        }
        fn normalize(v: Point<Px>) -> Point<Px> {
            let squared_len = v.x * v.x + v.y * v.y;
            let len = f32::sqrt(squared_len.into_float());
            v / len
        }

        if cross(v1, v2).abs() > epsilon {
            // this is not a straight line
            if path.is_empty() {
                path.push(PathEl::MoveTo(p2 + (normalize(v1) * -bevel)));
            } else {
                // vger_renderer doesn't implement LineTo ...
                path.push(PathEl::QuadTo(
                    p2 + (normalize(v1) * -bevel),
                    p2 + (normalize(v1) * -bevel),
                ));
            }
            path.push(PathEl::QuadTo(p2, p2 + (normalize(v2) * -bevel)));
        }
    }

    if let Some(PathEl::MoveTo(p)) = path.get(0).cloned() {
        // the path is not empty, close and return it
        path.push(PathEl::QuadTo(p, p));
        path.push(PathEl::Close);

        let mut p = PathBuilder::new(p);
        for el in path.iter().skip(1) {
            match el {
                PathEl::LineTo(point) => p = p.line_to(*point),
                PathEl::QuadTo(p1, p2) => p = p.quadratic_curve_to(*p1, *p2),
                // PathEl::Close => p = p.close(),
                // PathEl::MoveTo(_) => ()
                _ => (),
            }
        }
        Some(p.close())
    } else {
        None
    }
}
