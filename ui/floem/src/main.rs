// 🤦‍♀️😊❤😂🤣
use std::collections::HashMap;
use std::env::{self, Args};
use std::fmt::format;

use floem::cosmic_text::{Attrs, AttrsList, FamilyOwned, TextLayout, Wrap};
use floem::event::Event;
use floem::id::Id;
use floem::keyboard::{Key, NamedKey};
use floem::kurbo::{Point, Rect};
use floem::menu::{Menu, MenuEntry, MenuItem};
use floem::peniko::{Brush, Color};
use floem::reactive::{create_effect, create_rw_signal, create_signal, RwSignal};
use floem::view::{View, ViewData};
use floem::views::{container, h_stack, label, scroll, stack, v_stack, Decorators};
use floem::widgets::button;
use floem::{EventPropagation, Renderer};
use ndoc::{Document, Indentation};

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
}

pub fn text_editor(doc: impl Fn() -> RwSignal<Document> + 'static) -> TextEditor {
    let id = Id::next();
    let attrs = Attrs::new()
        .family(&[FamilyOwned::Monospace])
        .font_size(14.);

    let mut t = TextLayout::new();
    t.set_text(" ", AttrsList::new(attrs));
    let line_height = (t.lines[0].layout_opt().as_ref().unwrap()[0].line_ascent
        + t.lines[0].layout_opt().as_ref().unwrap()[0].line_descent) as f64;
    id.request_focus();
    TextEditor {
        data: ViewData::new(id),
        doc: doc(),
        text_node: None,
        viewport: Rect::default(),
        line_height,
        page_len: 0,
    }
}

impl TextEditor {
    pub fn scroll_to_main_cursor(&self) {
        self.id()
            .update_state(TextEditorCommand::FocusMainCursor, false);
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
                        let hit = t.hit_position(sel.column);
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
            layout.set_text(&l.to_string(), attr_list.clone());
            cx.draw_text(&layout, Point::new(0.5, y.ceil() + 0.5));

            if let Some(sel) = selection_areas.get(&i) {
                let start = layout.hit_position(sel.0);
                let end = layout.hit_position(sel.1);

                let r = Rect::new(start.point.x, y, end.point.x, y + self.line_height);
                let b = Brush::Solid(Color::BLACK.with_alpha_factor(0.5));

                cx.fill(&r, &b, 1.);
            }
            if let Some(sel) = selections.get(&i) {
                let pos = layout.hit_position(*sel);
                let r = Rect::new(
                    pos.point.x.ceil() + 0.5,
                    y.ceil() - 0.5,
                    pos.point.x.ceil() + 0.5,
                    (y + self.line_height).ceil() + 0.5,
                );
                let b = Brush::Solid(Color::BLACK);
                cx.stroke(&r, &b, 1.);
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
                dbg!(&e);
                match e.key.text {
                    Some(ref txt) if txt.chars().any(|c| !c.is_control()) => {
                        self.doc.update(|d| d.insert(txt));
                        self.scroll_to_main_cursor();
                        cx.request_paint(self.id());
                        EventPropagation::Stop
                    }
                    _ => match e.key.logical_key {
                        Key::Named(NamedKey::ArrowDown)
                            if e.modifiers.control_key() && e.modifiers.alt_key() =>
                        {
                            self.doc
                                .update(|d| d.duplicate_selection(ndoc::MoveDirection::Down));
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowUp)
                            if e.modifiers.control_key() && e.modifiers.alt_key() =>
                        {
                            self.doc
                                .update(|d| d.duplicate_selection(ndoc::MoveDirection::Up));
                            cx.request_paint(self.id());
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
                            cx.request_paint(self.id());
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
                            cx.request_paint(self.id());
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
                            cx.request_paint(self.id());
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
                            cx.request_paint(self.id());
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
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            self.doc.update(|d| {
                                d.move_selections(ndoc::MoveDirection::Up, e.modifiers.shift_key())
                            });
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Delete) => {
                            self.doc.update(|d| d.delete());
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Backspace) => {
                            self.doc.update(|d| d.backspace());
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab) if e.modifiers.shift_key() => {
                            self.doc.update(|d| d.deindent());
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab)
                            if self.doc.get().selections[0].is_single_line() =>
                        {
                            self.doc.update(|d| d.indent(false));
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab) => {
                            //dbg!(self.doc.file_info.indentation);
                            self.doc.update(|d| d.indent(true));
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::End) => {
                            self.doc.update(|d| d.end(e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Home) => {
                            self.doc.update(|d| d.home(e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Enter) => {
                            self.doc.update(|d| {
                                d.insert(&self.doc.get().file_info.linefeed.to_string())
                            });
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::PageUp) => {
                            self.doc
                                .update(|d| d.page_up(self.page_len, e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::PageDown) => {
                            self.doc
                                .update(|d| d.page_down(self.page_len, e.modifiers.shift_key()));
                            self.scroll_to_main_cursor();
                            cx.request_paint(self.id());

                            EventPropagation::Stop
                        }
                        _ => EventPropagation::Continue,
                    },
                }
            }
            Event::PointerDown(p) => {
                dbg!(p);
                EventPropagation::Continue
            }
            _ => EventPropagation::Continue,
        }
    }
}

fn editor(doc: impl Fn() -> RwSignal<Document> + 'static) -> impl View {
    let text_editor = text_editor(move || doc());
    let text_editor_id = text_editor.id();

    container(
        scroll(
            stack((text_editor
                .keyboard_navigatable()
                .on_click_stop(|e| {
                    dbg!(e.point());
                })
                .style(|s| s.size_full()),))
            .style(|s| s.padding(6.0)),
        )
        
        .style(|s| s.absolute().size_pct(100.0, 100.0)),
    )
    .on_click_cont(move |_| text_editor_id.request_focus())
    .style(move |s| s.border(1.0).border_radius(6.0).size_full())
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

    v.id().inspect();

    v
}

fn main() {
    floem::launch(app_view);
}
