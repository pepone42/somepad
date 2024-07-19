use std::collections::HashMap;
use std::time::{Duration, Instant};

use cushy::context::{AsEventContext, EventContext, GraphicsContext, WidgetContext};
use cushy::kludgine::app::winit::platform::windows::WindowExtWindows;
use cushy::kludgine::text::Text;
use cushy::value::{CallbackHandle, Dynamic, MapEach};

use cushy::figures::units::{self, Lp, Px, UPx};
use cushy::figures::{
    Abs, FloatConversion, Fraction, IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero,
};
use cushy::kludgine::app::winit::event::{ElementState, MouseButton};
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};
use cushy::kludgine::cosmic_text::{Attrs, Buffer, Cursor, Family, Metrics};
use cushy::kludgine::shapes::{Path, PathBuilder, Shape, StrokeOptions};
use cushy::kludgine::{Drawable, DrawableExt};

use cushy::styles::{components, Color};
use cushy::value::{Destination, Source};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetTag, WrapperWidget,
    HANDLED, IGNORED,
};

use cushy::widgets::Custom;
use cushy::{context, define_components, ModifiersExt, WithClone};
use ndoc::syntax::ThemeSetRegistry;
use ndoc::{Document, Position, Selection};
use rfd::FileDialog;
use scroll::ScrollController;

use crate::shortcut::{event_match, ModifiersCustomExt};
use crate::{get_settings, CommandsRegistry, FONT_SYSTEM};

use super::scroll::{self, ContextScroller, MyScroll, PassiveScroll};

pub struct CodeEditorColors {
    bg: Color,
    fg: Color,
    bg_selection: Color,
    border_selection: Color,
    cursor: Color,
    fg_gutter: Color,
    bg_gutter: Color,
    bg_find_hightlight: Color,
    
}

impl CodeEditorColors {
    pub fn get(kind: TextEditorKind, context: &GraphicsContext) -> Self {
        if kind == TextEditorKind::Code {
            let theme = &ThemeSetRegistry::get().themes[&get_settings().theme];
            let fg = theme
                .settings
                .foreground
                .map(|c| Color::new(c.r, c.g, c.b, c.a))
                .unwrap_or(context.get(&TextColor));
            let bg = theme
                .settings
                .background
                .map(|c| Color::new(c.r, c.g, c.b, c.a))
                .unwrap_or(context.get(&BackgroundColor));
            CodeEditorColors {
                bg,
                fg,
                bg_selection: theme
                    .settings
                    .selection
                    .map(|c| Color::new(c.r, c.g, c.b, c.a))
                    .unwrap_or(context.get(&SelectionBackgroundColor)),
                border_selection: theme
                    .settings
                    .selection_border
                    .map(|c| Color::new(c.r, c.g, c.b, c.a))
                    .unwrap_or(context.get(&SelectionBorderColor)),
                cursor: theme
                    .settings
                    .caret
                    .map(|c| Color::new(c.r, c.g, c.b, c.a))
                    .unwrap_or(context.get(&CursorColor)),
                fg_gutter: theme
                    .settings
                    .gutter_foreground
                    .map(|c| Color::new(c.r, c.g, c.b, c.a))
                    .unwrap_or(fg),
                bg_gutter: theme
                    .settings
                    .gutter
                    .map(|c| Color::new(c.r, c.g, c.b, c.a))
                    .unwrap_or(bg),
                bg_find_hightlight: theme.settings.find_highlight.map(|c| Color::new(c.r, c.g, c.b, c.a)).unwrap_or(context.get(&SelectionBackgroundColor)),
            }
        } else {
            CodeEditorColors {
                bg: context.get(&components::WidgetBackground),
                fg: context.get(&components::TextColor),
                bg_selection: context.get(&components::HighlightColor),
                border_selection: context.get(&components::HighlightColor),
                cursor: context.get(&components::OutlineColor),
                fg_gutter: Color::BLACK,
                bg_gutter: Color::WHITE,
                bg_find_hightlight: context.get(&components::HighlightColor),
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ClickInfo {
    count: usize,
    last_click: Option<Instant>,
    last_button: Option<MouseButton>,
}

impl ClickInfo {
    fn update(&mut self, button: MouseButton) {
        let now = Instant::now();
        match (self.last_click, self.last_button) {
            (Some(last_click), Some(last_button))
                if now - last_click < Duration::from_millis(300) && button == last_button =>
            {
                self.count += 1
            }
            _ => self.count = 0,
        }
        self.last_click = Some(now);
        self.last_button = Some(button);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEditorKind {
    Code,
    Input,
}

#[derive(Debug)]
pub struct TextEditor {
    pub doc: Dynamic<ndoc::Document>,
    viewport: Dynamic<Rect<Px>>,
    font_metrics: Metrics,
    font_size: Px,
    line_height: Px,
    scale: Fraction,
    cmd_reg: Dynamic<CommandsRegistry>,
    eol_width: Px,
    click_info: Dynamic<ClickInfo>,
    focused: Dynamic<bool>,
    kind: TextEditorKind,
    search_term: Dynamic<Document>,
    current_word: Dynamic<String>,
    foreach_handles: Vec<CallbackHandle>,
    items_found: Dynamic<Vec<(Position, Position)>>,
    current_words_found: Dynamic<Vec<(Position, Position)>>,
    current_search_item_idx: Dynamic<usize>,
    should_refocus: Dynamic<bool>,
    page_len: usize,
    search_panel_closed: Dynamic<bool>,
}

impl TextEditor {
    pub fn new(
        doc: Dynamic<ndoc::Document>,
        cmd_reg: Dynamic<CommandsRegistry>,
        click_info: Dynamic<ClickInfo>,
        search_term: Dynamic<Document>,
    ) -> Self {
        let mut editor = TextEditor::create(doc.clone());

        doc.lock().update_theme(&get_settings().theme);

        editor.cmd_reg = cmd_reg;
        editor.click_info = click_info;
        editor.search_term = search_term;

        let debounced_doc = editor.doc.debounced_with_delay(Duration::from_millis(500));

        editor.current_word = debounced_doc.map_each(move |d| {
            let pos = d.selections[0].head;
            let word_start = d.position_to_char(d.word_start(pos));
            let word_end = d.position_to_char(d.word_end(pos));
            d.rope.slice(word_start..word_end).to_string()
        });

        editor.items_found = editor.doc.with_clone(|doc| {
            editor.search_term.map_each(move |search_term| {
                let mut items = Vec::new();
                let mut idx = Position::new(0, 0);
                let search_term = search_term.rope.to_string();
                while let Some(i) = doc.get().find_from(&search_term, idx, false) {
                    items.push(i);
                    idx = i.1;
                }
                items
            })
        });

        // TODO: not preformant. We need a way to cancel a foreach if the doc changes while we are still iterating
        // editor.current_words_found = editor.doc.with_clone(|doc| {
        //     editor.current_word.map_each(move |current_word| {
        //         let mut items = Vec::new();
        //         let mut idx = Position::new(0, 0);
        //         while let Some(i) = doc.get().find_from(current_word, idx) {
        //             items.push(i);
        //             idx = i.1;
        //         }
        //         items
        //     })
        // });

        editor.foreach_handles.push(
            (&editor.doc, &editor.items_found, &editor.should_refocus).with_clone(
                |(doc, items_found, should_refocus)| {
                    editor
                        .current_search_item_idx
                        .for_each_cloned(move |seach_idx| {
                            if items_found.get().is_empty() {
                                return;
                            }
                            let (head, tail) = items_found.get()[seach_idx];
                            doc.lock().set_main_selection(head, tail);
                            should_refocus.replace(true);
                        })
                },
            ),
        );

        editor
    }

    pub fn as_input(doc: Dynamic<ndoc::Document>) -> Self {
        let mut editor = TextEditor::create(doc);
        editor.kind = TextEditorKind::Input;
        editor
    }

    fn create(doc: Dynamic<Document>) -> Self {
        Self {
            doc,
            viewport: Dynamic::new(Rect::default()),
            font_metrics: Default::default(),
            font_size: Px::ZERO,
            line_height: Px::ZERO,
            scale: Fraction::ZERO,
            cmd_reg: Dynamic::new(CommandsRegistry::new()),
            eol_width: Px::ZERO,
            click_info: Dynamic::new(ClickInfo::default()),
            focused: Dynamic::new(false),
            kind: TextEditorKind::Code,
            search_term: Dynamic::new(Document::default()),
            current_word: Dynamic::new(String::new()),
            foreach_handles: Vec::new(),
            items_found: Dynamic::new(Vec::new()),
            current_words_found: Dynamic::new(Vec::new()),
            current_search_item_idx: Dynamic::new(0),
            should_refocus: Dynamic::new(false),
            page_len: 0,
            search_panel_closed: Dynamic::new(false),
        }
    }

    fn px_to_col(&self, line: usize, x: Px) -> usize {
        let raw_text = self.doc.get().get_visible_line(line).to_string();
        let mut buffer = Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), self.font_metrics);
        buffer.set_size(
            &mut FONT_SYSTEM.lock().unwrap(),
            10000.,
            self.font_metrics.line_height,
        );
        buffer.set_text(
            &mut FONT_SYSTEM.lock().unwrap(),
            &raw_text,
            if self.kind == TextEditorKind::Code {
                Attrs::new().family(Family::Monospace)
            } else {
                Attrs::new()
            },
            cushy::kludgine::cosmic_text::Shaping::Advanced,
        );
        let byte_idx = buffer
            .hit(x.into_float(), self.font_metrics.line_height / 2.)
            .unwrap_or_default()
            .index;
        self.doc.get().byte_to_col(line, byte_idx)
    }

    fn col_to_px(&self, line: usize, index: usize) -> Px {
        let raw_text = self.doc.get().get_visible_line(line).to_string();
        let mut buffer = Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), self.font_metrics);
        buffer.set_size(
            &mut FONT_SYSTEM.lock().unwrap(),
            10000.,
            self.font_metrics.line_height,
        );
        buffer.set_text(
            &mut FONT_SYSTEM.lock().unwrap(),
            &raw_text,
            if self.kind == TextEditorKind::Code {
                Attrs::new().family(Family::Monospace)
            } else {
                Attrs::new()
            },
            cushy::kludgine::cosmic_text::Shaping::Advanced,
        );
        let col = self.doc.get().col_to_byte(line, index);
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

    pub fn refocus_main_selection(&self, context: &EventContext<'_>) {
        if self.doc.get().selections.len() == 1 {
            let main_selection_head_x = self.col_to_px(
                self.doc.get().selections[0].head.line,
                self.doc.get().selections[0].head.column,
            );
            context.make_region_visible(Rect::new(
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

    fn layout_line(&self, line_idx: usize) -> Buffer {
        let raw_text = self.doc.get().get_visible_line(line_idx).to_string();

        let attrs = if self.kind == TextEditorKind::Code {
            Attrs::new().family(Family::Monospace)
        } else {
            Attrs::new()
        };

        if let Some(sl) = self.doc.get().get_style_line_info(line_idx as _) {
            let mut buffer = Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), self.font_metrics);
            let mut spans = Vec::new();
            for s in sl.iter() {
                let start = s.range.start.min(raw_text.len());
                let end = s.range.end.min(raw_text.len());
                let t = &raw_text[start..end];

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
            buffer
        } else {
            let mut buffer = Buffer::new(&mut FONT_SYSTEM.lock().unwrap(), self.font_metrics);
            buffer.set_text(
                &mut FONT_SYSTEM.lock().unwrap(),
                &raw_text,
                attrs,
                cushy::kludgine::cosmic_text::Shaping::Advanced,
            );
            buffer.set_size(
                &mut FONT_SYSTEM.lock().unwrap(),
                10000.,
                self.font_metrics.line_height,
            );
            buffer
        }
    }

    fn get_selection_shape(
        &self,
        range: Selection,
        layouts: &HashMap<usize, Buffer>,
    ) -> Option<Path<Px, false>> {
        let rope = &self.doc.get().rope;

        let rects = range
            .areas(rope)
            .iter()
            .filter_map(|a| (layouts.contains_key(&a.line).then_some(*a)))
            .map(|a| {
                let col_start = self.doc.get().col_to_byte(a.line, a.col_start);
                let col_end = self.doc.get().col_to_byte(a.line, a.col_end);

                let c_start = Cursor::new(0, col_start);
                let c_end = if col_end == col_start {
                    Cursor::new(0, col_start + 1)
                } else {
                    Cursor::new(0, col_end)
                };

                let (start, end) = layouts[&a.line]
                    .layout_runs()
                    .nth(0)
                    .unwrap()
                    .highlight(c_start, c_end)
                    .unwrap_or_default();
                let start = start.into();
                let end = if col_end == col_start {
                    start
                } else {
                    start + Px::from_float(end)
                };

                let y = self.line_height * Px::new(a.line as i32);

                Rect::new(
                    Point::new(start, y),
                    Size::new(
                        end + if a.include_eol {
                            self.eol_width
                        } else {
                            Px::ZERO
                        } - start,
                        self.line_height,
                    ),
                )
            })
            .collect::<Vec<Rect<Px>>>();

        make_selection_path(&rects)
    }

    fn get_selections_shapes(&self, layouts: &HashMap<usize, Buffer>) -> Vec<Path<Px, false>> {
        self.doc
            .get()
            .selections
            .iter()
            .filter_map(|s| self.get_selection_shape(*s, layouts))
            .collect()
    }

    fn get_items_shapes(
        &self,
        items: Dynamic<Vec<(Position, Position)>>,
        layouts: &HashMap<usize, Buffer>,
    ) -> Vec<Path<Px, false>> {
        items
            .get()
            .iter()
            .filter_map(|s| {
                let range = (s.0, s.1).into();
                self.get_selection_shape(range, layouts)
            })
            .collect()
    }

    fn location_to_position(&self, location: Point<Px>) -> ndoc::Position {
        let line = ((self.viewport.get().origin.y + location.y) / self.line_height)
            .floor()
            .get();
        let line = (line.max(0) as usize).min(self.doc.get().rope.len_lines() - 1);
        let col_idx = self.px_to_col(line, location.x);
        Position::new(line, col_idx)
    }

    pub fn save_as(&self, context: &mut WidgetContext) {
        #[cfg(target_os = "windows")]
        context.window_mut().winit().unwrap().set_enable(false);
        if let Some(file) = FileDialog::new().save_file() {
            // TODO: check for errors
            let _ = self.doc.lock().save_as(&file);
        }
        #[cfg(target_os = "windows")]
        context.window_mut().winit().unwrap().set_enable(true);
        context.window_mut().winit().unwrap().focus_window();
    }
}

impl Widget for TextEditor {
    fn mounted(&mut self, context: &mut context::EventContext<'_>) {
        self.focused = context.widget.window_mut().focused().clone();
    }
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let colors = CodeEditorColors::get(self.kind, context);

        // So we can refocus easily from about anywere
        if self.should_refocus.get() {
            self.refocus_main_selection(&context.as_event_context());
            self.should_refocus.replace(false);
        }

        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.gfx.scale())
            .round();

        context.redraw_when_changed(&self.doc);
        context.redraw_when_changed(&self.current_words_found);

        if self.kind == TextEditorKind::Input && self.focused.get() {
            let focus_ring_color = context.get(&components::HighlightColor);
            let clip_rect = Rect::new(Point::ZERO, context.gfx.clip_rect().size).into_signed();
            context.gfx.draw_shape(
                Shape::stroked_rect(clip_rect, focus_ring_color).translate_by(Point::ZERO),
            );
        }

        let first_line = (-context.gfx.translation().y / self.line_height) - 1;
        let last_line =
            first_line + (context.gfx.clip_rect().size.height.into_signed() / self.line_height) + 2;

        let first_line = first_line.get().max(0) as usize;
        let last_line = last_line.get() as usize;
        let total_line = last_line - first_line;
        self.page_len =
            (context.gfx.clip_rect().size.height.into_signed() / self.line_height).get() as _;

        if self.kind == TextEditorKind::Code {
            context.gfx.set_font_size(Lp::points(12));
        }
        context.fill(colors.bg);
        let doc = self.doc.get();

        // TODO: cache layouts
        let buffers = self
            .doc
            .get()
            .rope
            .lines()
            .enumerate()
            .skip(first_line)
            .take(total_line)
            .map(|(i, _)| (i, self.layout_line(i)))
            .collect::<HashMap<usize, Buffer>>();

        // draw selections
        for path in self.get_selections_shapes(&buffers) {
            let bg_color = colors.bg_selection;
            let border_color = colors.border_selection;

            context.gfx.draw_shape(
                path.fill(bg_color)
                    .translate_by(Point::new(padding, padding)),
            );
            context.gfx.draw_shape(
                path.stroke(StrokeOptions::px_wide(Px::new(1)).colored(border_color))
                    .translate_by(Point::new(padding, padding)),
            );
        }

        // draw search items
        if !self.search_panel_closed.get() {
            for path in self.get_items_shapes(self.items_found.clone(), &buffers) {
                let bg_color = colors.bg_find_hightlight;

                context.gfx.draw_shape(
                    path.fill(bg_color)
                        .translate_by(Point::new(padding, padding)),
                );
            }
        } else {
            // for path in self.get_items_shapes(self.current_words_found.clone(),&buffers) {
            //     let bg_color = context.get(&SelectionBackgroundColor);
            //     let border_color = context.get(&SelectionBorderColor);

            //     context.gfx.draw_shape(
            //         path.fill(bg_color)
            //             .translate_by(Point::new(padding, padding)),
            //     );
            //     context.gfx.draw_shape(
            //         path.stroke(StrokeOptions::px_wide(Px::new(1)).colored(border_color))
            //             .translate_by(Point::new(padding, padding)),
            //     );
            // }
        }

        for i in first_line..last_line {
            let y = units::Px::new(i as _) * self.line_height;
            if let Some(b) = buffers.get(&i) {
                context.gfx.draw_text_buffer(
                    Drawable {
                        source: b,
                        translation: Point::<Px>::default(),
                        opacity: None,
                        rotation: None,
                        scale: None,
                    }
                    .translate_by(Point::new(padding, y + padding)),
                    colors.fg,
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
            let head = self.col_to_px(s.head.line, s.head.column).floor();

            context.gfx.draw_shape(
                Shape::filled_rect(
                    Rect::new(
                        Point::new(Px::ZERO, Px::ZERO),
                        Size::new(Px::new(1), self.line_height),
                    ),
                    colors.cursor,
                )
                .translate_by(Point::new(
                    head + padding,
                    Px::new(s.head.line as i32) * self.line_height + padding,
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
        context.gfx.reset_text_attributes();
        let font_size = context.get(&components::TextSize);
        context.gfx.set_font_size(font_size);
    }

    fn layout(
        &mut self,
        _available_space: Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_upx(context.gfx.scale())
            .round()
            * 2;

        if context.gfx.scale() != self.scale {
            self.scale = context.gfx.scale();
            self.line_height = context.get(&LineHeight).into_px(context.gfx.scale()).ceil();
            self.font_size = context.get(&TextSize).into_px(context.gfx.scale()).ceil();
            self.font_metrics =
                Metrics::new(self.font_size.into_float(), self.line_height.into_float());
            self.eol_width = context.gfx.measure_text("⏎").size.width;
        }
        let height = self.doc.get().rope.len_lines() as f32 * self.font_metrics.line_height;

        self.viewport.set(Rect::new(
            context.gfx.translation().abs(),
            context.gfx.size().into_signed(),
        ));

        context.gfx.reset_text_attributes();
        let font_size = context.get(&components::TextSize);
        context.gfx.set_font_size(font_size);
        Size::new(
            UPx::new(10000) + padding,
            UPx::new(height.ceil() as _) + padding,
        )
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
        button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let location = location - padding;
        if !context.enabled() {
            return IGNORED;
        }
        context.focus();

        if button == MouseButton::Left {
            self.click_info.lock().update(button);

            let pos = self.location_to_position(location);
            match self.click_info.get().count {
                0 => {
                    self.doc.lock().set_main_selection(pos, pos);
                }
                1 => {
                    self.doc.lock().select_word(pos);
                }
                2 => self.doc.lock().select_line(pos.line),
                _ => self.doc.lock().select_all(),
            }

            HANDLED
        } else {
            IGNORED
        }
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let location = location - padding;
        if button == MouseButton::Left {
            let head = self.location_to_position(location);
            match self.click_info.get().count {
                0 => {
                    let tail = self.doc.get().selections[0].tail;
                    self.doc.lock().set_main_selection(head, tail);
                }
                1 => self.doc.lock().expand_selection_by_word(head),
                2 => self.doc.lock().expand_selection_by_line(head),
                _ => (),
            }

            self.refocus_main_selection(context);
        }
    }

    fn keyboard_input(
        &mut self,
        _device_id: cushy::window::DeviceId,
        input: cushy::window::KeyEvent,
        _is_synthetic: bool,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {
        if !self.focused.get() {
            return IGNORED;
        }
        if !context.enabled() {
            return IGNORED;
        }

        if self.kind == TextEditorKind::Code
            && input.state == ElementState::Pressed
            && context.modifiers().possible_shortcut()
        {
            let v = self.cmd_reg.get().view_shortcut;
            let id = context.widget.widget().id();
            for (shortcut, cmd) in v.iter() {
                if event_match(&input, context.modifiers(), shortcut.clone()) {
                    (cmd.action)(id, self, context);
                    return HANDLED;
                }
            }
        }

        if event_match(&input, context.modifiers(), shortcut!(Ctrl + c)) {
            if let Some(mut clipboard) = context.cushy().clipboard_guard() {
                let _ = clipboard.set_text(self.doc.get().get_selection_content());
            }
            return HANDLED;
        }
        if event_match(&input, context.modifiers(), shortcut!(Ctrl + x)) {
            if let Some(mut clipboard) = context.cushy().clipboard_guard() {
                if !self.doc.get().get_selection_content().is_empty() {
                    let _ = clipboard.set_text(self.doc.get().get_selection_content());
                    self.doc.lock().insert("");
                    self.refocus_main_selection(context);
                }
            }
        }
        if event_match(&input, context.modifiers(), shortcut!(Ctrl + v)) {
            if let Some(mut clipboard) = context.cushy().clipboard_guard() {
                if let Ok(s) = clipboard.get_text() {
                    self.doc.lock().insert_many(&s);
                    self.refocus_main_selection(context);
                }
            }
        }

        if input.state == ElementState::Pressed && matches!(input.logical_key, Key::Named(_)) {
            match input.logical_key {
                Key::Named(NamedKey::Escape) => {
                    if self.kind == TextEditorKind::Code {
                        // TODO: clear selections
                        if self.doc.get().selections.len() > 1 {
                            self.doc.lock().cancel_multi_cursor();
                        } else if self.doc.get().selections[0].head
                            != self.doc.get().selections[0].tail
                        {
                            let mut d = self.doc.lock();
                            d.selections[0].tail = d.selections[0].head;
                        }
                        return HANDLED;
                    } else {
                        return IGNORED;
                    }
                }
                Key::Named(NamedKey::Backspace) => {
                    self.doc.lock().backspace();
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::Delete) => {
                    self.doc.lock().delete();
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowLeft) if context.modifiers().word_select() => {
                    self.doc.lock().move_selections_word(
                        ndoc::MoveDirection::Left,
                        context.modifiers().shift(),
                    );
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowRight) if context.modifiers().word_select() => {
                    self.doc.lock().move_selections_word(
                        ndoc::MoveDirection::Right,
                        context.modifiers().shift(),
                    );
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    self.doc
                        .lock()
                        .move_selections(ndoc::MoveDirection::Left, context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.doc
                        .lock()
                        .move_selections(ndoc::MoveDirection::Right, context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowUp) if self.kind == TextEditorKind::Code => {
                    self.doc
                        .lock()
                        .move_selections(ndoc::MoveDirection::Up, context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowDown) if self.kind == TextEditorKind::Code => {
                    self.doc
                        .lock()
                        .move_selections(ndoc::MoveDirection::Down, context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::Enter) => {
                    if self.kind == TextEditorKind::Code {
                        let linefeed = self.doc.get().file_info.linefeed.to_string();
                        self.doc.lock().insert(&linefeed);
                        self.refocus_main_selection(context);
                        return HANDLED;
                    } else {
                        return IGNORED;
                    }
                }
                Key::Named(NamedKey::End) => {
                    self.doc.lock().end(context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::Home) => {
                    self.doc.lock().home(context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::PageUp) => {
                    self.doc
                        .lock()
                        .page_up(self.page_len, context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::PageDown) => {
                    self.doc
                        .lock()
                        .page_down(self.page_len, context.modifiers().shift());
                    self.refocus_main_selection(context);
                    return HANDLED;
                }

                Key::Named(NamedKey::Tab)
                    if context.modifiers().shift() && self.kind == TextEditorKind::Code =>
                {
                    self.doc.lock().deindent();
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::Tab)
                    if self.doc.get().selections[0].is_single_line()
                        && !context.modifiers().ctrl()
                        && self.kind == TextEditorKind::Code =>
                {
                    self.doc.lock().indent(false);
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                Key::Named(NamedKey::Tab)
                    if !context.modifiers().ctrl() && self.kind == TextEditorKind::Code =>
                {
                    self.doc.lock().indent(true);
                    self.refocus_main_selection(context);
                    return HANDLED;
                }
                _ => {}
            }
        }

        match (input.state, input.text) {
            (ElementState::Pressed, Some(t)) if !context.modifiers().possible_shortcut() => {
                self.doc.lock().insert(&t);
                self.refocus_main_selection(context);

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
    font_metrics: Metrics,
    font_size: Px,
    line_height: Px,
    scale: Fraction,
    editor_id: WidgetId,
}

impl Gutter {
    pub fn new(doc: Dynamic<Document>, editor_id: WidgetId) -> Self {
        Self {
            doc,
            font_metrics: Metrics::new(15., 15.),
            font_size: Px::ZERO,
            line_height: Px::ZERO,
            scale: Fraction::ZERO,
            editor_id,
        }
    }
}

impl Widget for Gutter {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        let colors = CodeEditorColors::get(TextEditorKind::Code, context);
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.gfx.scale())
            .round();

        let first_line = (-context.gfx.translation().y / self.line_height) - 1;
        let last_line =
            first_line + (context.gfx.clip_rect().size.height.into_signed() / self.line_height) + 2;

        let first_line = first_line.get().max(0) as usize;
        let last_line = (last_line.get() as usize).min(self.doc.get().rope.len_lines());

        context
            .gfx
            .set_font_size(Px::new(self.font_metrics.font_size.ceil() as _));

        context.fill(colors.bg_gutter);

        for i in first_line..last_line {
            let y = units::Px::new(i as _) * self.font_metrics.line_height;

            let attrs = Attrs::new().family(Family::Monospace);

            context.gfx.set_text_attributes(attrs);

            context.gfx.draw_text(
                Text::new(&format!("{}", i + 1), colors.fg_gutter)
                    .translate_by(Point::new(Px::ZERO + padding, y + padding)),
            );
        }
    }
    fn layout(
        &mut self,
        _available_space: Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_upx(context.gfx.scale())
            .round()
            * 2;

        if context.gfx.scale() != self.scale {
            self.scale = context.gfx.scale();
            self.line_height = context.get(&LineHeight).into_px(context.gfx.scale()).ceil();
            self.font_size = context.get(&TextSize).into_px(context.gfx.scale()).ceil();
            self.font_metrics =
                Metrics::new(self.font_size.into_float(), self.line_height.into_float());
        }
        let height = self.doc.get().rope.len_lines() as f32 * self.font_metrics.line_height;

        context
            .gfx
            .set_font_size(Px::new(self.font_metrics.font_size.ceil() as _));
        let attrs = Attrs::new().family(Family::Monospace);

        context.gfx.set_text_attributes(attrs);

        let mesured_text = context
            .gfx
            .measure_text::<UPx>(&format!("{}", self.doc.get().rope.len_lines() + 1));
        let width = mesured_text.size.width;

        context.gfx.reset_text_attributes();
        let font_size = context.get(&components::TextSize);
        context.gfx.set_font_size(font_size);

        Size::new(
            width + padding,
            UPx::new(height.ceil().into_signed() as _) + padding,
        )
    }
    fn full_control_redraw(&self) -> bool {
        true
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
        button: MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let location = location - padding;
        if button == MouseButton::Left {
            let c = context.for_other(&self.editor_id).unwrap();
            let guard = c.widget().lock();
            let editor = guard.downcast_ref::<TextEditor>().unwrap();

            let line = (location.y / editor.line_height).floor().get();
            let line = (line.max(0) as usize).min(editor.doc.get().rope.len_lines() - 1);

            editor.doc.lock().select_line(line);
            HANDLED
        } else {
            IGNORED
        }
    }
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: cushy::window::DeviceId,
        button: MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) {
        let padding = context
            .get(&components::IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let location = location - padding;
        if button == MouseButton::Left {
            let c = context.for_other(&self.editor_id).unwrap();
            let guard = c.widget().lock();
            let editor = guard.downcast_ref::<TextEditor>().unwrap();
            let line = (location.y / editor.line_height).floor().get();
            let line = (line.max(0) as usize).min(editor.doc.get().rope.len_lines() - 1);

            editor
                .doc
                .lock()
                .expand_selection_by_line(Position::new(line, 0));
            editor.refocus_main_selection(&c);
        }
    }
}

#[derive(Debug)]
pub struct CodeEditor {
    child: cushy::widget::WidgetRef,
    search_id: WidgetId,
    pub editor_id: WidgetId,
    collapse_search_panel: Dynamic<bool>,
}

impl CodeEditor {
    pub fn new(doc: Dynamic<Document>, cmd_reg: Dynamic<CommandsRegistry>) -> Self {
        let search_term = Dynamic::new(Document::default());
        let show_search_panel: Dynamic<bool> = Dynamic::new(true);
        let (editor_tag, editor_id) = WidgetTag::new();
        let (search_tag, search_id) = WidgetTag::new();
        let scroller = Dynamic::new(ScrollController::default());
        let click_info = Dynamic::new(ClickInfo::default());
        let mut text_editor =
            TextEditor::new(doc.clone(), cmd_reg, click_info, search_term.clone());
        text_editor.search_panel_closed = show_search_panel.clone();
        let nb_searched_item = (
            &search_term,
            &text_editor.current_search_item_idx,
            &text_editor.items_found,
        )
            .map_each(|(s, a, b)| {
                if b.is_empty() {
                    "0/0".to_string()
                } else if s.rope.len_chars() > 0 {
                    format!("{}/{}", a + 1, b.len())
                } else {
                    "".to_string()
                }
            });

        let search_match = text_editor.items_found.map_each(|s| !s.is_empty());

        let action_up = (
            &text_editor.current_search_item_idx,
            &text_editor.items_found,
        )
            .with_clone(|(idx, items)| {
                move || {
                    if items.get().is_empty() {
                        return;
                    }
                    *idx.lock() = (idx.get() + items.get().len() - 1) % items.get().len();
                }
            });
        let action_down = (
            &text_editor.current_search_item_idx,
            &text_editor.items_found,
        )
            .with_clone(|(idx, items)| {
                move || {
                    if items.get().is_empty() {
                        return;
                    }
                    *idx.lock() = (idx.get() + 1) % items.get().len();
                }
            });
        let action_enter = action_down.clone();

        let child =
            (PassiveScroll::vertical(Gutter::new(doc.clone(), editor_id), scroller.clone())
                .and(
                    MyScroll::new(text_editor.make_with_tag(editor_tag))
                        .with_controller(scroller.clone())
                        .expand(),
                )
                .into_columns()
                .gutter(Px::new(1)))
            .expand_vertically()
            .and(
                "Search: "
                    .and(MyScroll::horizontal(
                        Custom::new(
                            TextEditor::as_input(search_term.clone())
                                .make_with_tag(search_tag)
                                .width(Lp::cm(5)),
                        )
                        .on_keyboard_input(move |_, k, _, _| {
                            if k.state == ElementState::Pressed
                                && k.logical_key == Key::Named(NamedKey::Enter)
                            {
                                action_enter();
                                HANDLED
                            } else {
                                IGNORED
                            }
                        }),
                    ))
                    .and(nb_searched_item)
                    .and(
                        "↑"
                            .into_button()
                            .on_click(move |_| action_up())
                            .with_enabled(search_match.clone()),
                    )
                    .and(
                        "↓"
                            .into_button()
                            .on_click(move |_| {
                                action_down()
                                // TODO: refocus
                            })
                            .with_enabled(search_match.clone()),
                    )
                    .into_columns()
                    .collapse_vertically(show_search_panel.clone()),
            )
            .into_rows();
        Self {
            child: child.widget_ref(),
            editor_id,
            search_id,
            collapse_search_panel: show_search_panel,
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

    fn keyboard_input(
        &mut self,
        _device_id: cushy::window::DeviceId,
        input: cushy::window::KeyEvent,
        _is_synthetic: bool,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if input.state == ElementState::Pressed
            && event_match(&input, context.modifiers(), shortcut!(Ctrl + f))
        {
            self.collapse_search_panel.toggle();
            if self.collapse_search_panel.get() {
                context.for_other(&self.editor_id).unwrap().focus();
            } else {
                context.for_other(&self.search_id).unwrap().focus();
            }
            return HANDLED;
        }
        IGNORED
    }
}

define_components! {
    CodeEditor {
        TextSize(Lp, "text_size", Lp::points(11))
        TextColor(Color, "text_color", Color::new(0xFF, 0xFF, 0xFF, 0xFF))
        LineHeight(Lp, "line_height", Lp::points(13))
        BackgroundColor(Color, "background_color", Color::new(0x34, 0x3D, 0x46, 0xFF))
        GutterBackgroundColor(Color, "gutter_background", Color::new(0x34, 0x3D, 0x46, 0xFF))
        GutterForegroundColor(Color, "gutter_foreground", Color::new(0xFF, 0xFF, 0xFF, 0xFF))
        CursorColor(Color, "cursor_color", Color::new(0xFF, 0xFF, 0xFF, 0xFF))
        SelectionBackgroundColor(Color, "selection_background_color", Color::new(0x4F, 0x5B, 0x66, 0xFF))
        SelectionBorderColor(Color, "selection_border_color", Color::new(0x20, 0x30, 0x40, 0xFF))
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
        let x0 = r.origin.x.floor();
        let y0 = r.origin.y.floor();
        let x1 = (r.origin.x + r.size.width).ceil();
        let y1 = (r.origin.y + r.size.height).ceil();
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
                path.push(PathEl::LineTo(p2 + (normalize(v1) * -bevel)));
            }
            path.push(PathEl::QuadTo(p2, p2 + (normalize(v2) * -bevel)));
        }
    }

    if let Some(PathEl::MoveTo(p)) = path.first().cloned() {
        // the path is not empty, close and return it
        path.push(PathEl::QuadTo(p, p));
        path.push(PathEl::Close);

        let mut p = PathBuilder::new(p);
        for el in path.iter().skip(1) {
            match el {
                PathEl::LineTo(point) => p = p.line_to(*point),
                PathEl::QuadTo(p1, p2) => p = p.quadratic_curve_to(*p1, *p2),
                _ => (),
            }
        }
        Some(p.close())
    } else {
        None
    }
}
