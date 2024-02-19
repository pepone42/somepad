// 🤦‍♀️😊❤😂🤣
use std::collections::HashMap;
use std::env::{self, Args};
use std::fmt::format;
use std::iter::empty;

use floem::cosmic_text::fontdb::Database;
use floem::cosmic_text::{Attrs, AttrsList, Family, FamilyOwned, TextLayout, Wrap};
use floem::event::Event;
use floem::id::Id;
use floem::keyboard::{Key, NamedKey};
use floem::kurbo::{Point, Rect};
use floem::menu::{Menu, MenuEntry, MenuItem};
use floem::peniko::{Brush, Color};
use floem::reactive::{create_effect, create_rw_signal, create_signal, RwSignal};
use floem::style::Position;
use floem::taffy::layout::{self, Layout};
use floem::view::{View, ViewData};
use floem::views::{
    container, h_stack, label, list, scroll, stack, text, v_stack, virtual_list, virtual_stack,
    Decorators, VirtualItemSize,
};
use floem::widgets::button;
use floem::{EventPropagation, Renderer};
use ndoc::rope_utils::{
    byte_to_grapheme, char_to_grapheme, grapheme_to_byte, grapheme_to_char, NextGraphemeIdxIterator,
};
use ndoc::{Document, Indentation, Selection};

enum TextEditorCommand {
    FocusMainCursor,
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

// pub struct Gutter {
//     data: ViewData,
//     doc: RwSignal<Document>,
//     text_node: Option<floem::taffy::prelude::Node>,
//     line_height: f64,
// }

// impl View for Gutter {
//     fn view_data(&self) -> &ViewData {
//         &self.data
//     }

//     fn view_data_mut(&mut self) -> &mut ViewData {
//         &mut self.data
//     }

//     fn layout(&mut self, cx: &mut floem::context::LayoutCx) -> floem::taffy::prelude::Node {
//         cx.layout_node(self.id(), true, |cx| {
//             let (width, height) = (
//                 100.,
//                 self.line_height * self.doc.get().rope.len_lines() as f64,
//             ); //attrs.line_height. * self.rope.len_lines());

//             if self.text_node.is_none() {
//                 self.text_node = Some(
//                     cx.app_state_mut()
//                         .taffy
//                         .new_leaf(floem::taffy::style::Style::DEFAULT)
//                         .unwrap(),
//                 );
//             }
//             let text_node = self.text_node.unwrap();
//             let style = floem::style::Style::new()
//                 .width(width)
//                 .height(height)
//                 .to_taffy_style();
//             let _ = cx.app_state_mut().taffy.set_style(text_node, style);

//             vec![text_node]
//         })
//     }
// }

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
            .update_state(TextEditorCommand::FocusMainCursor, true);
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
}

impl View for TextEditor {
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
                        // let idx = NextGraphemeIdxIterator::new(&self.doc.get().rope.line(sel.line)).nth(sel.column);
                        // let hit = t.hit_position(idx.unwrap());
                        let hit = t.hit_position(grapheme_to_byte(
                            &self.doc.get().rope.line(sel.line),
                            sel.column,
                        ));
                        let rect = Rect::new(
                            hit.point.x - 25.,
                            self.line_height * (sel.line as f64) - 25.,
                            hit.point.x + 25.,
                            self.line_height * ((sel.line + 1) as f64) + 25.,
                        );
                        self.id().scroll_to(Some(rect));
                    }
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

        let selections = self
            .doc
            .get()
            .selections
            .iter()
            .flat_map(|s| s.areas(&self.doc.get().rope))
            .collect::<Vec<(usize, usize, usize)>>();
        let mut selection_areas = HashMap::new();
        for s in selections {
            selection_areas.insert(s.2, (s.0, s.1));
        }
        let selections = self
            .doc
            .get()
            .selections
            .iter()
            .map(|s| (s.head.line, s.head.column))
            .collect::<HashMap<usize, usize>>();

        let mut y = (first_line as f64) * self.line_height;
        for (i, l) in self
            .doc
            .get()
            .rope
            .lines()
            .enumerate()
            .skip(first_line)
            .take(total_line)
        {
            layout.set_tab_width(self.doc.get().file_info.indentation.len());
            layout.set_text(&l.to_string(), attr_list.clone());

            if let Some(sel) = selection_areas.get(&i) {
                let start =
                    layout.hit_position(grapheme_to_byte(&self.doc.get().rope.line(i), sel.0));
                let end =
                    layout.hit_position(grapheme_to_byte(&self.doc.get().rope.line(i), sel.1));

                let r = Rect::new(
                    start.point.x.ceil(),
                    y.ceil(),
                    end.point.x.ceil(),
                    (y + self.line_height).ceil() + 1.,
                );
                let b = Brush::Solid(Color::GRAY);

                cx.fill(&r, &b, 0.);
            }

            cx.draw_text(&layout, Point::new(0., y.ceil()));

            if let Some(sel) = selections.get(&i) {
                let pos = layout.hit_position(grapheme_to_byte(&self.doc.get().rope.line(i), *sel));
                let r = Rect::new(
                    pos.point.x.ceil(),
                    y.ceil(),
                    pos.point.x.ceil(),
                    (y + self.line_height).ceil(),
                );
                let b = Brush::Solid(Color::BLACK);
                cx.stroke(&r, &b, 2.);
            }

            y += self.line_height;
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
                    EventPropagation::Stop
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
                        },
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
                //    list((0..dbg!(line_len.get())).map(|i| label(move || format!(" {} ", i.to_string())).style(|s| s.font_family("Monospace".to_string()).font_size(14.)))),
                virtual_stack(
                    floem::views::VirtualDirection::Vertical,
                    VirtualItemSize::Fixed(Box::new(move || text_editor.line_height)),
                    move || line_numbers.get(),
                    move |item| *item,
                    |i| label(move || format!(" {} ", (i + 1).to_string())), // .style(|s| {
                                                                             //     s.color(Color::BLACK)
                                                                             //         .font_family("Monospace".to_string())
                                                                             //         .font_size(14.)
                                                                             // })
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
        .style(|s| s.padding(6.)), //.flex().gap(10.,0.)),//.height(24.).min_height(24.)),
    ))
    .style(|s| s.width_full().height_full().font_size(14.))
    .window_title(|| "xncode".to_string());

    //v.id().inspect();

    v
}

fn main() {
    floem::launch(app_view);
}
