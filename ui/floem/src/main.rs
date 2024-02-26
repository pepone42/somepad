// 🤦‍♀️😊❤😂🤣
use std::collections::HashMap;
use std::env::{self, Args};
use std::fmt::format;
use std::iter::empty;

use floem::cosmic_text::fontdb::Database;
use floem::cosmic_text::{
    Attrs, AttrsList, Family, FamilyOwned, Font, HitPosition, TextLayout, Wrap,
};
use floem::event::Event;
use floem::id::Id;
use floem::keyboard::{Key, ModifiersState, NamedKey};
use floem::kurbo::{BezPath, PathEl, Point, Rect};
use floem::menu::{Menu, MenuEntry, MenuItem};
use floem::peniko::{Brush, Color};
use floem::reactive::{create_effect, create_rw_signal, create_signal, RwSignal};
use floem::style::{FontFamily, Position, StyleProp};
use floem::taffy::layout::{self, Layout};
use floem::view::{View, ViewData, Widget};
use floem::views::{
    container, h_stack, label, list, scroll, stack, text, v_stack, virtual_list, virtual_stack,
    Decorators, VirtualItemSize,
};
use floem::widgets::button;
use floem::{EventPropagation, Renderer};
use ndoc::rope_utils::{
    byte_to_grapheme, char_to_grapheme, grapheme_to_byte, grapheme_to_char, NextGraphemeIdxIterator,
};
use ndoc::{Document, Indentation, Rope, Selection, SelectionAera};

enum TextEditorCommand {
    FocusMainCursor,
    SelectLine(usize),
}

pub struct TextEditor {
    data: ViewData,
    doc: RwSignal<Document>,
    text_node: Option<floem::taffy::prelude::Node>,
    viewport: Rect,
    line_height: f64,
    page_len: usize,
    char_base_width: f64,
    selection_kind: SelectionKind,
}

pub enum SelectionKind {
    Char,
    Word,
    Line,
}

pub fn text_editor(doc: impl Fn() -> RwSignal<Document> + 'static) -> TextEditor {
    let id = Id::next();
    let attrs = Attrs::new()
        .family(&[FamilyOwned::Monospace])
        .font_size(14.);

    let mut t = TextLayout::new();
    t.set_text("8", AttrsList::new(attrs));
    let line_height = (t.lines[0].layout_opt().as_ref().unwrap()[0].line_ascent
        + t.lines[0].layout_opt().as_ref().unwrap()[0].line_descent) as f64;

    let char_base_width = t.lines[0].layout_opt().as_ref().unwrap()[0].w as f64;
    id.request_focus();
    TextEditor {
        data: ViewData::new(id),
        doc: doc(),
        text_node: None,
        viewport: Rect::default(),
        line_height,
        page_len: 0,
        char_base_width,
        selection_kind: SelectionKind::Char,
    }
}
impl TextEditor {
    pub fn scroll_to_main_cursor(&self) {
        self.id()
            .update_state_deferred(TextEditorCommand::FocusMainCursor);
    }

    pub fn layout_line(&self, line: usize) -> TextLayout {
        let mut layout = TextLayout::new();
        let attrs = Attrs::new()
            .color(Color::BLACK)
            .family(&[FamilyOwned::Monospace])
            .font_size(14.);

        let attr_list = AttrsList::new(attrs);
        layout.set_tab_width(self.doc.get().file_info.indentation.len());
        layout.set_text(&self.doc.get().rope.line(line).to_string(), attr_list);
        layout
    }

    pub fn point_to_position(&self, point: Point) -> ndoc::Position {
        let line = ((point.y / self.line_height) as usize).min(self.doc.get().rope.len_lines() - 1);
        let layout = self.layout_line(line);

        let col = layout.hit_point(Point::new(point.x, self.line_height / 2.0));
        let col = byte_to_grapheme(&self.doc.get().rope.line(line), col.index);
        ndoc::Position::new(line, col)
    }

    fn get_selection_shape(
        &self,
        selection: Selection,
        layouts: &HashMap<usize, TextLayout>,
    ) -> floem::kurbo::BezPath {
        let rope = &self.doc.get().rope;

        let rects = selection
            .areas(rope)
            .iter()
            .filter_map(|a| (layouts.contains_key(&a.line).then_some(*a)))
            .map(|a| {
                let start = self.hit_position(&layouts[&a.line], a.line, a.col_start);
                let end = self.hit_position(&layouts[&a.line], a.line, a.col_end);
                let y = a.line as f64 * self.line_height;

                Rect::new(
                    start.point.x.ceil(),
                    y.ceil(),
                    end.point.x.ceil()
                        + if a.include_eol {
                            self.char_base_width.ceil()
                        } else {
                            0.0
                        },
                    (y + self.line_height).ceil(),
                )
            })
            .collect::<Vec<Rect>>();

        make_selection_path(&rects)
    }

    fn get_selections_shapes(
        &self,
        layouts: &HashMap<usize, TextLayout>,
    ) -> Vec<floem::kurbo::BezPath> {
        self.doc
            .get()
            .selections
            .iter()
            .map(|s| self.get_selection_shape(*s, layouts))
            .collect()
    }

    fn hit_position(&self, layout: &TextLayout, line: usize, col: usize) -> HitPosition {
        layout.hit_position(grapheme_to_byte(&self.doc.get().rope.line(line), col))
    }
}

fn make_selection_path(aera: &[Rect]) -> floem::kurbo::BezPath {
    let mut path = BezPath::new();
    let bevel: f64 = 3.0;
    let epsilon: f64 = 0.0001;

    match aera {
        [] => {}
        [r] => {
            if r.x1 - r.x0 > epsilon {
                path.push(PathEl::MoveTo(Point::new(r.x0, r.y1 - bevel)));
                path.push(PathEl::QuadTo(
                    Point::new(r.x0, r.y0 + bevel),
                    Point::new(r.x0, r.y0 + bevel),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(r.x0, r.y0),
                    Point::new(r.x0 + bevel, r.y0),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(r.x1 - bevel, r.y0),
                    Point::new(r.x1 - bevel, r.y0),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(r.x1, r.y0),
                    Point::new(r.x1, r.y0 + bevel),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(r.x1, r.y1 - bevel),
                    Point::new(r.x1, r.y1 - bevel),
                ));

                path.push(PathEl::QuadTo(
                    Point::new(r.x1, r.y1),
                    Point::new(r.x1 - bevel, r.y1),
                ));

                path.push(PathEl::QuadTo(
                    Point::new(r.x0 + bevel, r.y1),
                    Point::new(r.x0 + bevel, r.y1),
                ));

                path.push(PathEl::QuadTo(
                    Point::new(r.x0, r.y1),
                    Point::new(r.x0, r.y1 - bevel),
                ));

                path.push(PathEl::QuadTo(
                    Point::new(r.x0, r.y1 - bevel),
                    Point::new(r.x0, r.y1 - bevel),
                ));
            }
        }
        [f, aera @ ..] => {
            path.push(PathEl::MoveTo(Point::new(f.x0, f.y1 - bevel)));
            path.push(PathEl::QuadTo(
                Point::new(f.x0, f.y0 + bevel),
                Point::new(f.x0, f.y0 + bevel),
            ));
            path.push(PathEl::QuadTo(
                Point::new(f.x0, f.y0),
                Point::new(f.x0 + bevel, f.y0),
            ));
            path.push(PathEl::QuadTo(
                Point::new(f.x1 - bevel, f.y0),
                Point::new(f.x1 - bevel, f.y0),
            ));
            path.push(PathEl::QuadTo(
                Point::new(f.x1, f.y0),
                Point::new(f.x1, f.y0 + bevel),
            ));
            path.push(PathEl::QuadTo(
                Point::new(f.x1, f.y1 - bevel),
                Point::new(f.x1, f.y1 - bevel),
            ));

            let mut last = (f.x1, f.y1, f.y0);

            for r in aera {
                let delta = if r.x1 < last.0 {
                    -bevel
                } else if r.x1 - last.0 < epsilon {
                    0.0
                } else {
                    bevel
                };

                path.push(PathEl::QuadTo(
                    Point::new(last.0, r.y0),
                    Point::new(last.0 + delta, r.y0),
                ));

                path.push(PathEl::QuadTo(
                    Point::new(r.x1 - delta, r.y0),
                    Point::new(r.x1 - delta, r.y0),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(r.x1, r.y0),
                    Point::new(r.x1, r.y0 + bevel),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(r.x1, r.y1 - bevel),
                    Point::new(r.x1, r.y1 - bevel),
                ));

                last = (r.x1, r.y1, r.y0);
            }

            if last.0 < epsilon {
                path.pop();
                path.pop();

                path.push(PathEl::QuadTo(
                    Point::new(bevel, last.2),
                    Point::new(bevel, last.2),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(0., last.2),
                    Point::new(0., last.2 - bevel),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(0., f.y1 + bevel),
                    Point::new(0., f.y1 + bevel),
                ));
            } else {
                path.push(PathEl::QuadTo(
                    Point::new(last.0, last.1),
                    Point::new(last.0 - bevel, last.1),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(bevel, last.1),
                    Point::new(bevel, last.1),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(0., last.1),
                    Point::new(0., last.1 - bevel),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(0., f.y1 + bevel),
                    Point::new(0., f.y1 + bevel),
                ));
            }
            if f.x0 > epsilon {
                path.push(PathEl::QuadTo(
                    Point::new(0., f.y1),
                    Point::new(bevel, f.y1),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(f.x0 - bevel, f.y1),
                    Point::new(f.x0 - bevel, f.y1),
                ));
                path.push(PathEl::QuadTo(
                    Point::new(f.x0, f.y1),
                    Point::new(f.x0, f.y1 - bevel),
                ));
            } else {
                path.push(PathEl::QuadTo(
                    Point::new(f.x0, f.y1 - bevel),
                    Point::new(f.x0, f.y1 - bevel),
                ));
            }

            path.close_path();
        }
    }

    path
}

impl View for TextEditor {
    fn view_data(&self) -> &ViewData {
        &self.data
    }

    fn view_data_mut(&mut self) -> &mut ViewData {
        &mut self.data
    }

    fn build(self) -> floem::view::AnyWidget {
        Box::new(self)
    }
}

impl Widget for TextEditor {
    fn view_data(&self) -> &ViewData {
        &self.data
    }

    fn view_data_mut(&mut self) -> &mut ViewData {
        &mut self.data
    }

    fn update(&mut self, _cx: &mut floem::context::UpdateCx, state: Box<dyn std::any::Any>) {
        if let Ok(cmd) = state.downcast::<TextEditorCommand>() {
            match *cmd {
                TextEditorCommand::FocusMainCursor => {
                    if self.doc.get().selections.len() == 1 {
                        let sel = self.doc.get().selections[0].head;
                        let attrs = Attrs::new()
                            .family(&[FamilyOwned::Monospace])
                            .font_size(14.);

                        let mut t = TextLayout::new();
                        t.set_text(
                            &self.doc.get().rope.line(sel.line).to_string(),
                            AttrsList::new(attrs),
                        );

                        let hit = self.hit_position(&t, sel.line, sel.column);

                        let rect = Rect::new(
                            hit.point.x - 25.,
                            self.line_height * (sel.line as f64) - 25.,
                            hit.point.x + 25.,
                            self.line_height * ((sel.line + 1) as f64) + 25.,
                        );
                        self.id().scroll_to(Some(rect));
                    }
                }
                TextEditorCommand::SelectLine(line) => {
                    self.doc.update(|d| d.select_line(line));
                }
            }
        }
    }

    fn layout(&mut self, cx: &mut floem::context::LayoutCx) -> floem::taffy::prelude::Node {
        cx.layout_node(self.id(), true, |cx| {
            let (width, height) = (
                1024.,
                self.line_height * self.doc.get().rope.len_lines() as f64,
            ); //attrs.line_height. * self.rope.len_lines());

            if self.text_node.is_none() {
                self.text_node = Some(
                    cx.app_state_mut()
                        .taffy
                        .new_leaf(floem::taffy::style::Style::DEFAULT)
                        .unwrap(),
                );
            }
            let text_node = self.text_node.unwrap();
            let style = floem::style::Style::new()
                .width(width)
                .height(height)
                .to_taffy_style();
            let _ = cx.app_state_mut().taffy.set_style(text_node, style);

            vec![text_node]
        })
    }

    fn compute_layout(&mut self, cx: &mut floem::context::ComputeLayoutCx) -> Option<Rect> {
        self.viewport = cx.current_viewport();
        self.page_len = (self.viewport.height() / self.line_height).ceil() as usize;
        None
    }

    fn paint(&mut self, cx: &mut floem::context::PaintCx) {
        let mut layout = TextLayout::new();
        let attrs = Attrs::new()
            .color(Color::BLACK)
            .family(&[FamilyOwned::Monospace])
            .font_size(14.);

        let attr_list = AttrsList::new(attrs);

        let first_line = ((self.viewport.y0 / self.line_height).ceil() as usize).saturating_sub(1);
        let total_line = ((self.viewport.height() / self.line_height).ceil() as usize) + 1;

        layout.set_tab_width(self.doc.get().file_info.indentation.len());
        let layouts = self
            .doc
            .get()
            .rope
            .lines()
            .enumerate()
            .skip(first_line)
            .take(total_line)
            .map(|(i, l)| {
                layout.set_text(&l.to_string(), attr_list.clone());
                (i, layout.clone())
            })
            .collect::<HashMap<usize, TextLayout>>();

        let selections = self
            .doc
            .get()
            .selections
            .iter()
            .map(|s| (s.head.line, s.head.column))
            .collect::<HashMap<usize, usize>>();

        // Draw Selections
        for path in self.get_selections_shapes(&layouts) {
            let bg = Brush::Solid(Color::DARK_BLUE.with_alpha_factor(0.25));
            let fg = Brush::Solid(Color::DARK_BLUE);
            if !path.is_empty() {
                cx.fill(&path, &bg, 0.);
                cx.stroke(&path, &fg, 0.5);
            }
        }

        for (i, layout) in layouts {
            let y = i as f64 * self.line_height;
            // Draw Text
            cx.draw_text(&layout, Point::new(0., y.ceil()));

            // Draw Cursor
            if let Some(sel) = selections.get(&i) {
                let pos = self.hit_position(&layout, i, *sel);
                let r = Rect::new(
                    pos.point.x.ceil(),
                    y.ceil(),
                    pos.point.x.ceil(),
                    (y + self.line_height).ceil(),
                );
                let b = Brush::Solid(Color::BLACK);
                cx.stroke(&r, &b, 2.);
            }
        }
    }

    fn event(
        &mut self,
        cx: &mut floem::context::EventCx,
        _id_path: Option<&[Id]>,
        event: floem::event::Event,
    ) -> floem::EventPropagation {
        //dbg!(event.clone());
        match event {
            Event::KeyDown(e) => {
                //dbg!(&e);
                match e.key.text {
                    Some(ref txt) if txt.chars().any(|c| !c.is_control()) => {
                        self.doc.update(|d| d.insert(txt));
                        self.scroll_to_main_cursor();
                        cx.request_all(self.id());
                        EventPropagation::Stop
                    }
                    _ => match e.key.logical_key {
                        Key::Named(NamedKey::ArrowDown)
                            if e.modifiers.control_key() && e.modifiers.alt_key() =>
                        {
                            self.doc
                                .update(|d| d.duplicate_selection(ndoc::MoveDirection::Down));
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowUp)
                            if e.modifiers.control_key() && e.modifiers.alt_key() =>
                        {
                            self.doc
                                .update(|d| d.duplicate_selection(ndoc::MoveDirection::Up));
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowLeft) if e.modifiers.control_key() => {
                            self.doc.update(|d| {
                                d.move_selections_word(
                                    ndoc::MoveDirection::Left,
                                    e.modifiers.shift_key(),
                                )
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowRight) if e.modifiers.control_key() => {
                            self.doc.update(|d| {
                                d.move_selections_word(
                                    ndoc::MoveDirection::Right,
                                    e.modifiers.shift_key(),
                                )
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            self.doc.update(|d| {
                                d.move_selections(
                                    ndoc::MoveDirection::Left,
                                    e.modifiers.shift_key(),
                                )
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            self.doc.update(|d| {
                                d.move_selections(
                                    ndoc::MoveDirection::Right,
                                    e.modifiers.shift_key(),
                                )
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            self.doc.update(|d| {
                                d.move_selections(
                                    ndoc::MoveDirection::Down,
                                    e.modifiers.shift_key(),
                                )
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            self.doc.update(|d| {
                                d.move_selections(ndoc::MoveDirection::Up, e.modifiers.shift_key())
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Delete) => {
                            self.doc.update(|d| d.delete());
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Backspace) => {
                            self.doc.update(|d| d.backspace());
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab) if e.modifiers.shift_key() => {
                            self.doc.update(|d| d.deindent());
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab)
                            if self.doc.get().selections[0].is_single_line() =>
                        {
                            self.doc.update(|d| d.indent(false));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab) => {
                            //dbg!(self.doc.file_info.indentation);
                            self.doc.update(|d| d.indent(true));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::End) => {
                            self.doc.update(|d| d.end(e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Home) => {
                            self.doc.update(|d| d.home(e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Enter) => {
                            let line_feed = self.doc.get().file_info.linefeed.to_string();
                            self.doc.update(|d| d.insert(&line_feed));

                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::PageUp) => {
                            self.doc
                                .update(|d| d.page_up(self.page_len, e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::PageDown) => {
                            self.doc
                                .update(|d| d.page_down(self.page_len, e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());

                            EventPropagation::Stop
                        }
                        _ => EventPropagation::Continue,
                    },
                }
            }
            Event::PointerDown(p) => {
                if p.button.is_primary() {
                    let position = self.point_to_position(p.pos);

                    match p.count {
                        1 => {
                            self.doc.update(|d| {
                                d.selections = vec![Selection::new(position.line, position.column)]
                            });
                            self.selection_kind = SelectionKind::Char;
                        }
                        2 => {
                            self.doc.update(|d| d.select_word(position));
                            self.selection_kind = SelectionKind::Word;
                        }
                        3 => {
                            self.doc.update(|d| d.select_line(position.line));
                            self.selection_kind = SelectionKind::Line;
                        }
                        4 => {
                            self.doc.update(|d| d.select_all());
                        }
                        _ => (),
                    }

                    cx.update_active(self.id());
                    EventPropagation::Continue // EventPropagation::Stop here stop all keyboard event to occur ...
                } else {
                    EventPropagation::Continue
                }
            }
            Event::PointerMove(p) => {
                if cx.is_active(self.id()) {
                    let position = self.point_to_position(p.pos);
                    match self.selection_kind {
                        SelectionKind::Char => {
                            self.doc.update(|d| d.selections[0].head = position);
                        }
                        SelectionKind::Word => {
                            self.doc.update(|d| d.expand_selection_by_word(position));
                        }
                        SelectionKind::Line => {
                            self.doc.update(|d| d.expand_selection_by_line(position));
                        }
                    }

                    self.scroll_to_main_cursor();
                    EventPropagation::Stop
                } else {
                    EventPropagation::Continue
                }
            }
            _ => EventPropagation::Continue,
        }
    }
}

fn editor(doc: impl Fn() -> RwSignal<Document> + 'static) -> impl View {
    let ndoc = doc(); //.clone();
    let text_editor = text_editor(move || ndoc);
    let text_editor_id = text_editor.id();
    let (line_number_width, line_number_width_set) = create_signal(
        (ndoc.get().rope.len_lines().to_string().len() + 2) as f64 * text_editor.char_base_width,
    );
    let (line_numbers, line_numbers_set) =
        create_signal((0..(ndoc.get().rope.len_lines() + 1)).collect::<im::Vector<usize>>());
    create_effect(move |_| {
        line_numbers_set.set((0..ndoc.get().rope.len_lines()).collect::<im::Vector<usize>>());
        line_number_width_set.set(
            (ndoc.get().rope.len_lines().to_string().len() + 2) as f64
                * text_editor.char_base_width,
        );
    });

    container(
        scroll(
            h_stack((
                virtual_stack(
                    floem::views::VirtualDirection::Vertical,
                    VirtualItemSize::Fixed(Box::new(move || text_editor.line_height)),
                    move || line_numbers.get(),
                    move |item| *item,
                    move |i| {
                        label(move || format!(" {} ", (i + 1).to_string())).on_event_cont(
                            floem::event::EventListener::PointerDown,
                            move |_| {
                                text_editor_id
                                    .update_state_deferred(TextEditorCommand::SelectLine(i))
                            },
                        )
                    },
                )
                .style(move |s| {
                    s.color(Color::BLACK)
                        .font_family("Monospace".to_string())
                        .font_size(14.)
                        .width(line_number_width.get())
                }),
                floem::views::empty().style(|s| s.width(1.0).background(Color::BLACK)),
                text_editor
                    .keyboard_navigatable()
                    .style(|s| s.size_full().margin_left(5.)),
            ))
            .style(|s| s.padding(6.0)),
        )
        .style(|s| s.padding(1.0).absolute().size_pct(100.0, 100.0)),
    )
    .on_click_cont(move |_| text_editor_id.request_focus())
    .style(move |s| s.border(1.0).border_radius(1.0).size_full())
}

fn indentation_menu(indent: impl Fn(Indentation) + 'static + Clone) -> Menu {
    let mut tab = Menu::new("Tabs");
    let mut space = Menu::new("Spaces");
    // tab.entry(MenuItem::new("1".to_string()).action(move || (indent.clone())(Indentation::Tab(1))));
    // let ind = indent.clone();
    // tab.entry(MenuItem::new("2".to_string()).action(move || (indent.clone())(Indentation::Tab(2))));
    for i in 1..9 {
        let ind = indent.clone();
        tab = tab.entry(MenuItem::new(i.to_string()).action(move || ind(Indentation::Tab(i))));
        let ind = indent.clone();
        space =
            space.entry(MenuItem::new(i.to_string()).action(move || ind(Indentation::Space(i))));
    }
    Menu::new("Indentation").entry(tab).entry(space)
}

fn app_view() -> impl View {
    let ndoc = if let Some(path) = env::args().nth(1) {
        ndoc::Document::from_file(path).unwrap()
    } else {
        ndoc::Document::default()
    };

    let (indentation, set_indentation) = create_signal(ndoc.file_info.indentation);
    let doc = create_rw_signal(ndoc);

    create_effect(move |_| {
        doc.update(|d| d.file_info.indentation = indentation.get());
    });

    let v = v_stack((
        editor(move || doc),
        //label(|| "label".to_string()).style(|s| s.size_full()),
        h_stack((
            label(move || match doc.get().file_name {
                Some(f) => f.file_name().unwrap().to_string_lossy().to_string(),
                None => "[Untilted]".to_string(),
            })
            .style(|s| s.width_full()), //.height(24.)),
            label(move || indentation.get().to_string())
                .popout_menu(move || indentation_menu(move |indent| set_indentation.set(indent)))
                .style(|s| {
                    s.padding_left(10.)
                        .padding_right(10.)
                        .hover(|s| s.background(Color::BLACK.with_alpha_factor(0.15)))
                }),
            label(move || doc.get().file_info.encoding.name().to_string()),
        ))
        .style(|s| s.padding(6.)),
    ))
    .style(|s| s.width_full().height_full().font_size(12.))
    .window_title(|| "xncode".to_string());

    let id = v.id();

    v.keyboard_navigatable().on_key_down(
        Key::Named(NamedKey::F10),
        ModifiersState::empty(),
        move |_| id.inspect(),
    )
}

fn main() {
    floem::launch(app_view);
}
