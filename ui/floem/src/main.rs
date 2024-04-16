// 🤦‍♀️😊❤😂🤣
mod command;
mod decorators;
mod documents;
#[macro_use]
mod shortcut;
mod settings;
mod theme;
mod widgets;

use anyhow::Context;
use command::{ViewCommand, WindowCommand};
use documents::Documents;
use floem::action::{add_overlay, open_file, remove_overlay, save_as};
use floem::cosmic_text::{Attrs, AttrsList, FamilyOwned, HitPosition, TextLayout};
use floem::event::Event;
use floem::ext_event::create_signal_from_channel;
use floem::file::{FileDialogOptions, FileSpec};
use floem::id::Id;
use floem::keyboard::{Key, Modifiers, ModifiersState, NamedKey};
use floem::kurbo::{BezPath, PathEl, Point, Rect};
use floem::menu::{Menu, MenuItem};
use floem::peniko::{Brush, Color};
use floem::reactive::{create_effect, create_rw_signal, create_signal, RwSignal};
use once_cell::sync::Lazy;
use settings::Settings;
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::raw;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex};
use std::{env, time};
use window::{get_id_path, WindowUpdateCommand};

use floem::view::{View, ViewData, Widget};
use floem::views::{
    container, dyn_container, empty, h_stack, label, scroll, v_stack, virtual_list, virtual_stack,
    Decorators, VirtualDirection, VirtualItemSize,
};

use floem::{Application, Clipboard, EventPropagation, Renderer};
use ndoc::rope_utils::{byte_to_grapheme, grapheme_to_byte};

use ndoc::theme::THEME;
use ndoc::{Document, Indentation, Selection};
use shortcut::{event_match, Shortcut};
use widgets::{palette_free, Palette};

use crate::widgets::palette_list;
use crate::widgets::window;

pub fn color_syntect_to_peniko(col: ndoc::Color) -> Color {
    Color::rgba8(col.r, col.g, col.b, col.a)
}

pub static FOCUSED_TEXT_EDITOR: AtomicU64 = AtomicU64::new(0);

pub fn focused_editor() -> Id {
    let raw_id = FOCUSED_TEXT_EDITOR.load(std::sync::atomic::Ordering::Relaxed);
    let id = unsafe { std::mem::transmute::<u64, Id>(raw_id) };
    id
}

#[derive(Debug)]
enum TextEditorCommand {
    FocusMainCursor,
    SelectLine(usize),
}

pub struct TextEditor {
    data: ViewData,
    doc: RwSignal<Document>,
    text_node: Option<floem::taffy::tree::NodeId>,
    viewport: Rect,
    line_height: f64,
    page_len: usize,
    char_base_width: f64,
    selection_kind: SelectionKind,
    disable: RwSignal<bool>,
    multiline: bool,
    on_change: Option<Box<dyn Fn()>>,
    on_arrow_up: Option<Box<dyn Fn()>>,
    on_arrow_down: Option<Box<dyn Fn()>>,
    on_return: Option<Box<dyn Fn()>>,
    on_escape: Option<Box<dyn Fn()>>,
}
#[derive(Debug, Clone, Copy)]
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
    FOCUSED_TEXT_EDITOR.store(id.to_raw(), std::sync::atomic::Ordering::Relaxed);

    let disable = RwSignal::new(false);

    let ndoc = doc();

    // create a system to update the view when the highlighter is updated
    // on_lighter_update is called from an other thread
    // we need to send a message to the main thread to update the view
    let (s, r) = crossbeam_channel::unbounded();
    let x = create_signal_from_channel(r);
    ndoc.get().on_highlighter_update(move || {
        s.send(()).unwrap();
    });

    create_effect(move |_| match x.get() {
        _ => {
            id.request_paint();
        }
    });

    create_effect(move |_| {
        if disable.get() {
            id.clear_focus();
        }
    });

    TextEditor {
        data: ViewData::new(id),
        doc: ndoc,
        text_node: None,
        viewport: Rect::default(),
        line_height,
        page_len: 0,
        char_base_width,
        selection_kind: SelectionKind::Char,
        disable,
        multiline: true,
        on_change: None,
        on_arrow_up: None,
        on_arrow_down: None,
        on_return: None,
        on_escape: None,
    }
    .disabled(move || disable.get())
}
impl TextEditor {
    pub fn multiline(mut self, value: bool) -> Self {
        self.multiline = value;
        self
    }

    pub fn on_change(mut self, f: impl Fn() + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }
    pub fn on_arrow_down(mut self, f: impl Fn() + 'static) -> Self {
        self.on_arrow_down = Some(Box::new(f));
        self
    }
    pub fn on_arrow_up(mut self, f: impl Fn() + 'static) -> Self {
        self.on_arrow_up = Some(Box::new(f));
        self
    }
    pub fn on_return(mut self, f: impl Fn() + 'static) -> Self {
        self.on_return = Some(Box::new(f));
        self
    }
    pub fn on_escape(mut self, f: impl Fn() + 'static) -> Self {
        self.on_escape = Some(Box::new(f));
        self
    }

    pub fn scroll_to_main_cursor(&self) {
        self.id()
            .update_state_deferred(TextEditorCommand::FocusMainCursor);
    }

    pub fn layout_line(&self, line: usize) -> TextLayout {
        let mut layout = TextLayout::new();
        let attrs = Attrs::new()
            .color(Color::parse(&THEME.vscode.colors.editor_foreground).unwrap())
            .family(&[FamilyOwned::Monospace])
            .font_size(14.);
        let mut attr_list = AttrsList::new(attrs);

        if let Some(style) = self.doc.get().get_style_line_info(line) {
            //self.highlighted_line.get(line) {
            for s in style.iter() {
                let fg = Color::rgba8(
                    s.style.foreground.r,
                    s.style.foreground.g,
                    s.style.foreground.b,
                    s.style.foreground.a,
                );
                attr_list.add_span(
                    s.range.clone(),
                    Attrs::new()
                        .color(fg)
                        .family(&[FamilyOwned::Monospace])
                        .font_size(14.),
                );
            }
        }

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
    ) -> Option<floem::kurbo::BezPath> {
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
            .filter_map(|s| self.get_selection_shape(*s, layouts))
            .collect()
    }

    fn hit_position(&self, layout: &TextLayout, line: usize, col: usize) -> HitPosition {
        layout.hit_position(grapheme_to_byte(&self.doc.get().rope.line(line), col))
    }

    fn save_as(&self) {
        let doc = self.doc.clone();
        save_as(
            FileDialogOptions::new()
                .default_name("new.txt")
                .title("Save file"),
            move |file_info| {
                if let Some(file) = file_info {
                    doc.update(|d| d.save_as(&file.path[0]).unwrap());
                }
            },
        );
    }
}

fn make_selection_path(rects: &[Rect]) -> Option<floem::kurbo::BezPath> {
    let bevel: f64 = 3.0;
    let epsilon: f64 = 0.0001;

    let mut left = Vec::with_capacity(rects.len() * 2);
    let mut right = Vec::with_capacity(rects.len() * 2);
    for r in rects.iter().filter(|r| r.x1 - r.x0 > epsilon) {
        right.push(Point::new(r.x1, r.y0));
        right.push(Point::new(r.x1, r.y1));
        left.push(Point::new(r.x0, r.y0));
        left.push(Point::new(r.x0, r.y1));
    }
    left.reverse();

    let points = [right, left].concat();
    let mut path = BezPath::new();

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

        if v1.cross(v2).abs() > epsilon {
            // this is not a straight line
            if path.is_empty() {
                path.push(PathEl::MoveTo(p2 + (v1.normalize() * -bevel)));
            } else {
                // vger_renderer doesn't implement LineTo ...
                path.push(PathEl::QuadTo(
                    p2 + (v1.normalize() * -bevel),
                    p2 + (v1.normalize() * -bevel),
                ));
            }
            path.push(PathEl::QuadTo(p2, p2 + (v2.normalize() * -bevel)));
        }
    }

    if let Some(PathEl::MoveTo(p)) = path.elements().get(0) {
        // the path is not empty, close and return it
        path.push(PathEl::QuadTo(*p, *p));
        path.close_path();
        Some(path)
    } else {
        None
    }
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

    fn layout(&mut self, cx: &mut floem::context::LayoutCx) -> floem::taffy::tree::NodeId {
        cx.layout_node(self.id(), true, |cx| {
            let (mut width, height) = (
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

            if !self.multiline {
                let layout = self.layout_line(0);
                width = layout.size().width;
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
        let first_line = ((self.viewport.y0 / self.line_height).ceil() as usize).saturating_sub(1);
        let total_line = ((self.viewport.height() / self.line_height).ceil() as usize) + 1;

        let layouts = self
            .doc
            .get()
            .rope
            .lines()
            .enumerate()
            .skip(first_line)
            .take(total_line)
            .map(|(i, _)| (i, self.layout_line(i)))
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
            let bg_color = &THEME.vscode.colors.selection_background;
            let border_color =
                color_art::Color::from_hex(&THEME.vscode.colors.selection_background)
                    .unwrap()
                    .darken(0.1)
                    .hex_full();

            let bg = Brush::Solid(Color::parse(bg_color).unwrap());
            let fg = Brush::Solid(Color::parse(&border_color).unwrap());

            if !path.is_empty() {
                cx.fill(&path, &bg, 0.);
                cx.stroke(&path, &fg, 1.);
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
        id_path: Option<&[Id]>,
        event: floem::event::Event,
    ) -> floem::EventPropagation {
        VIEW_SHORTCUT.with(|v| {
            for (shortcut, cmd) in v.borrow().iter() {
                if event_match(&event, shortcut.clone()) {
                    (cmd.action)(self);
                    return EventPropagation::Stop;
                }
            }
            EventPropagation::Continue
        });

        // Keyboard event does not bubble up to the root view
        // we circumvent that by capturing event here, and sending update_state
        WINDOW_SHORTCUT.with(|v| {
            for (shortcut, cmd) in v.borrow().iter() {
                if event_match(&event, shortcut.clone()) {
                    let mut path = get_id_path(self.id());
                    path.reverse();
                    if let Some(root_id) = path.get(1) {
                        root_id.update_state_deferred(WindowUpdateCommand::LaunchCommand(
                            cmd.id.to_string(),
                        ))
                    }
                }
            }
            EventPropagation::Continue
        });

        match event {
            Event::FocusGained => {
                dbg!(self.id());
                FOCUSED_TEXT_EDITOR.store(self.id().to_raw(), std::sync::atomic::Ordering::Relaxed);
                EventPropagation::Continue
            }
            Event::KeyDown(e) => {
                match e.key.text {
                    Some(ref txt) if txt.chars().any(|c| !c.is_control()) => {
                        match txt.as_str() {
                            _ => {
                                if !e.modifiers.control()
                                    && !e.modifiers.alt()
                                    && !e.modifiers.meta()
                                {
                                    self.doc.update(|d| d.insert(txt));
                                    self.scroll_to_main_cursor();
                                    cx.request_all(self.id());
                                    if let Some(action) = self.on_change.as_ref() {
                                        action();
                                    }
                                    return EventPropagation::Stop;
                                }
                            }
                        }
                        EventPropagation::Continue
                    }
                    _ => match e.key.logical_key {
                        Key::Named(NamedKey::ArrowDown)
                            if e.modifiers.control() && e.modifiers.alt() && self.multiline =>
                        {
                            self.doc
                                .update(|d| d.duplicate_selection(ndoc::MoveDirection::Down));
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowUp)
                            if e.modifiers.control() && e.modifiers.alt() && self.multiline =>
                        {
                            self.doc
                                .update(|d| d.duplicate_selection(ndoc::MoveDirection::Up));
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowLeft) if e.modifiers.control() => {
                            self.doc.update(|d| {
                                d.move_selections_word(
                                    ndoc::MoveDirection::Left,
                                    e.modifiers.shift(),
                                )
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowRight) if e.modifiers.control() => {
                            self.doc.update(|d| {
                                d.move_selections_word(
                                    ndoc::MoveDirection::Right,
                                    e.modifiers.shift(),
                                )
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            self.doc.update(|d| {
                                d.move_selections(ndoc::MoveDirection::Left, e.modifiers.shift())
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            self.doc.update(|d| {
                                d.move_selections(ndoc::MoveDirection::Right, e.modifiers.shift())
                            });
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            if let Some(action) = self.on_arrow_down.as_ref() {
                                action();
                            } else {
                                self.doc.update(|d| {
                                    d.move_selections(
                                        ndoc::MoveDirection::Down,
                                        e.modifiers.shift(),
                                    )
                                });
                                self.scroll_to_main_cursor();
                                cx.request_all(self.id());
                            }
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if let Some(action) = self.on_arrow_up.as_ref() {
                                action();
                            } else {
                                self.doc.update(|d| {
                                    d.move_selections(ndoc::MoveDirection::Up, e.modifiers.shift())
                                });
                                self.scroll_to_main_cursor();
                                cx.request_all(self.id());
                            }
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Delete) => {
                            self.doc.update(|d| d.delete());
                            if let Some(action) = self.on_change.as_ref() {
                                action();
                            }
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Backspace) => {
                            self.doc.update(|d| d.backspace());
                            if let Some(action) = self.on_change.as_ref() {
                                action();
                            }
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab) if e.modifiers.shift() && self.multiline => {
                            self.doc.update(|d| d.deindent());
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab)
                            if self.doc.get().selections[0].is_single_line() && self.multiline =>
                        {
                            self.doc.update(|d| d.indent(false));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Tab) if self.multiline => {
                            //dbg!(self.doc.file_info.indentation);
                            self.doc.update(|d| d.indent(true));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::End) => {
                            self.doc.update(|d| d.end(e.modifiers.shift()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());
                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Home) => {
                            self.doc.update(|d| d.home(e.modifiers.shift()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Enter) => {
                            if let Some(action) = self.on_return.as_ref() {
                                action();
                            } else if self.multiline {
                                let line_feed = self.doc.get().file_info.linefeed.to_string();
                                self.doc.update(|d| d.insert(&line_feed));

                                self.scroll_to_main_cursor();
                                cx.request_all(self.id());
                            }

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Escape) => {
                            if let Some(action) = self.on_escape.as_ref() {
                                action();
                                EventPropagation::Stop
                            } else {
                                EventPropagation::Continue
                            }
                        }
                        Key::Named(NamedKey::PageUp) if self.multiline => {
                            self.doc
                                .update(|d| d.page_up(self.page_len, e.modifiers.shift()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::PageDown) if self.multiline => {
                            self.doc
                                .update(|d| d.page_down(self.page_len, e.modifiers.shift()));
                            self.scroll_to_main_cursor();
                            cx.request_all(self.id());

                            EventPropagation::Stop
                        }
                        Key::Named(NamedKey::Escape) => {
                            self.doc.update(|d| {
                                d.cancel_multi_cursor();
                            });
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
                                d.set_main_selection(position, position);
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
                    self.id().request_paint();
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
                    s.color(Color::parse(&THEME.vscode.colors.editor_foreground).unwrap())
                        .font_family("Monospace".to_string())
                        .font_size(14.)
                        .width(line_number_width.get())
                }),
                floem::views::empty().style(|s| {
                    s.width(1.0)
                        .background(Color::parse(&THEME.vscode.colors.editor_foreground).unwrap())
                }),
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
    .style(|s| s.background(Color::parse(&THEME.vscode.colors.editor_background).unwrap()))
}

fn startup_screen() -> impl View {
    container(v_stack((
        label(move || "Welcome to SomePad".to_string()),
        label(move || "Ctrl+n to create a new document".to_string()).style(|s| s.font_size(18.)),
        label(move || "Ctrl+o to open an existing file".to_string()).style(|s| s.font_size(18.)),
    )))
    .style(|s| {
        s.size_full()
            .flex_row()
            .items_center()
            .justify_center()
            .font_size(24.)
            .border(1.0)
            .border_color(Color::BLACK)
    })
}

fn indentation_menu(indent: impl Fn(Indentation) + 'static + Clone) -> Menu {
    let mut tab = Menu::new("Tabs");
    let mut space = Menu::new("Spaces");
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
    ndoc::Document::init_highlighter();

    let documents = create_rw_signal(Documents::new());

    if let Some(path) = env::args().nth(1) {
        let doc = create_rw_signal(Document::from_file(path).unwrap());
        documents.update(|docs| docs.add(doc));
    }

    let v = window(
        v_stack((
            dyn_container(
                move || documents.get().current_id(),
                move |val| {
                    if documents.get().is_empty() {
                        startup_screen().any()
                    } else {
                        editor(move || documents.get().get_doc(val)).any()
                    }
                },
            )
            .style(|s| s.size_full()),
            h_stack((
                label(move || {
                    if documents.get().is_empty() {
                        "".to_string()
                    } else {
                        match documents.get().current().get().file_name {
                            Some(f) => f.file_name().unwrap().to_string_lossy().to_string(),
                            None => "[Untilted]".to_string(),
                        }
                    }
                }),
                label(move || {
                    if documents.get().is_empty() {
                        "".to_string()
                    } else {
                        if documents.get().current().get().is_dirty() {
                            "●".to_string()
                        } else {
                            "".to_string()
                        }
                    }
                })
                .style(|s| s.width_full()), //.height(24.)),
                label(move || {
                    if documents.get().is_empty() {
                        "".to_string()
                    } else {
                        documents
                            .get()
                            .current()
                            .get()
                            .file_info
                            .indentation
                            .to_string()
                    }
                })
                .popout_menu(move || {
                    if documents.get().is_empty() {
                        Menu::new("nothing")
                    } else {
                        indentation_menu(move |indent| {
                            documents
                                .get()
                                .current()
                                .update(|d| d.file_info.indentation = indent)
                        })
                    }
                })
                .style(|s| {
                    s.padding_left(10.)
                        .padding_right(10.)
                        .hover(|s| s.background(Color::BLACK.with_alpha_factor(0.15)))
                }),
                label(move || {
                    if documents.get().is_empty() {
                        "".to_string()
                    } else {
                        documents
                            .get()
                            .current()
                            .get()
                            .file_info
                            .encoding
                            .name()
                            .to_string()
                    }
                }),
            ))
            .style(|s| s.padding(6.)),
        ))
        .style(|s| s.size_full()),
        documents,
    )
    .style(|s| {
        s.width_full()
            .height_full()
            .font_size(12.)
            .color(Color::parse(&THEME.vscode.colors.editor_foreground).unwrap())
            .background(Color::parse(&THEME.vscode.colors.editor_background).unwrap())
    })
    .style(move |s| s)
    .window_title(|| "xncode".to_string());

    let id = v.id();

    v.keyboard_navigatable()
        .on_key_down(Key::Named(NamedKey::F10), Modifiers::empty(), move |_| {
            id.inspect()
        })
}

thread_local! {
    pub static VIEW_SHORTCUT: RefCell<HashMap<Shortcut,ViewCommand>> = RefCell::new(HashMap::new());
    pub static VIEW_COMMAND_REGISTRY: RefCell<HashMap<&'static str, ViewCommand>> = RefCell::new(HashMap::new());

    pub static WINDOW_SHORTCUT: RefCell<HashMap<Shortcut,WindowCommand>> = RefCell::new(HashMap::new());
    pub static WINDOW_COMMAND_REGISTRY: RefCell<HashMap<&'static str, WindowCommand>> = RefCell::new(HashMap::new());
}

const GOTOLINE_CMD: ViewCommand = ViewCommand {
    name: "Go To Line",
    id: "editor.gotoline",
    action: |v| {
        let doc = v.doc.clone();
        let id = v.id();
        palette_free(v.id(), move |line| {
            doc.update(move |d| {
                if let Ok(line_idx) = line.parse::<usize>() {
                    let line_idx = line_idx.min(d.rope.len_lines()).max(1);
                    let pos = d.char_to_position(d.rope.line_to_char(line_idx - 1));
                    d.set_main_selection(pos, pos);
                }
            });
            id.update_state_deferred(TextEditorCommand::FocusMainCursor);
        });
    },
};

const COPY_SELECTION_CMD: ViewCommand = ViewCommand {
    name: "Copy Selection",
    id: "editor.copyselection",
    action: |v| {
        let _ = Clipboard::set_contents(v.doc.get().get_selection_content());
    },
};

const CUT_SELECTION_CMD: ViewCommand = ViewCommand {
    name: "Cut Selection",
    id: "editor.cutselection",
    action: |v| {
        if v.doc.get().get_selection_content().len() > 0 {
            let _ = Clipboard::set_contents(v.doc.get().get_selection_content());
            v.doc.update(|d| d.insert(""));
            v.scroll_to_main_cursor();
        }
    },
};

const PASTE_SELECTION_CMD: ViewCommand = ViewCommand {
    name: "Paste Selection",
    id: "editor.pasteselection",
    action: |v| {
        if let Ok(s) = Clipboard::get_contents() {
            v.doc.update(|d| d.insert_many(&s));
            v.scroll_to_main_cursor();
        }
    },
};

const SAVE_DOC_AS_CMD: ViewCommand = ViewCommand {
    name: "Save Document As",
    id: "editor.savedocas",
    action: |v| {
        v.save_as();
    },
};

const SAVE_DOC_CMD: ViewCommand = ViewCommand {
    name: "Save Document",
    id: "editor.savedoc",
    action: |v| {
        if let Some(ref file_name) = v.doc.get().file_name {
            v.doc.update(|d| d.save_as(file_name).unwrap());
        } else {
            v.save_as();
        }
    },
};

const UNDO_CMD: ViewCommand = ViewCommand {
    name: "Undo",
    id: "editor.undo",
    action: |v| {
        v.doc.update(|d| d.undo());
        v.scroll_to_main_cursor();
    },
};
const REDO_CMD: ViewCommand = ViewCommand {
    name: "redo",
    id: "editor.redo",
    action: |v| {
        v.doc.update(|d| d.redo());
        v.scroll_to_main_cursor();
    },
};

const NEW_DOC: WindowCommand = WindowCommand {
    name: "New Document",
    id: "window.newdoc",
    action: |w| {
        let doc = create_rw_signal(Document::new(get_settings().indentation));
        w.documents.update(|docs| docs.add(doc));
    },
};

const OPEN_DOC: WindowCommand = WindowCommand {
    name: "Open Document",
    id: "window.opendoc",
    action: |w| {
        let doc = create_rw_signal(Document::new(get_settings().indentation));
        let documents = w.documents.clone();
        open_file(FileDialogOptions::new().title("Open new file"), move |p| {
            if let Some(path) = p {
                doc.set(Document::from_file(&path.path[0]).unwrap());
                documents.update(|d| d.add(doc));
                //disabled.set(false);
            }
        });
    },
};

const CLOSE_DOC: WindowCommand = WindowCommand {
    name: "Close Document",
    id: "window.closedoc",
    action: |w| {
        w.documents.update(|d| d.remove(d.current_id()));
    },
};

const SHOW_OPENED_DOC: WindowCommand = WindowCommand {
    name: "Show opened docuements",
    id: "window.opendeddocs",
    action: |w| {
        let documents = w.documents.clone();
        if !documents.get().is_empty() {
            w.id().palette(
                //viewport,
                documents
                    .get()
                    .order_by_mru()
                    .iter()
                    .enumerate()
                    .map(|(_, d)| (d.get().id(), d.get().title().to_string())),
                move |i| {
                    documents.update(|d| d.set_current(i));
                    //disabled.set(false);
                },
            );
        }
    },
};

pub static SETTINGS: Lazy<Arc<Mutex<Settings>>> =
    Lazy::new(|| Arc::new(Mutex::new(Settings::load())));

pub fn get_settings() -> Settings {
    SETTINGS.lock().unwrap().clone()
}

fn main() -> anyhow::Result<()> {
    VIEW_COMMAND_REGISTRY.with(|v| {
        v.borrow_mut().insert(GOTOLINE_CMD.id, GOTOLINE_CMD);
        v.borrow_mut()
            .insert(COPY_SELECTION_CMD.id, COPY_SELECTION_CMD);
        v.borrow_mut()
            .insert(PASTE_SELECTION_CMD.id, PASTE_SELECTION_CMD);
        v.borrow_mut()
            .insert(CUT_SELECTION_CMD.id, CUT_SELECTION_CMD);
        v.borrow_mut().insert(SAVE_DOC_CMD.id, SAVE_DOC_CMD);
        v.borrow_mut().insert(SAVE_DOC_AS_CMD.id, SAVE_DOC_AS_CMD);
        v.borrow_mut().insert(UNDO_CMD.id, UNDO_CMD);
        v.borrow_mut().insert(REDO_CMD.id, REDO_CMD);
    });
    WINDOW_COMMAND_REGISTRY.with(|w| {
        w.borrow_mut().insert(NEW_DOC.id, NEW_DOC);
        w.borrow_mut().insert(OPEN_DOC.id, OPEN_DOC);
        w.borrow_mut().insert(CLOSE_DOC.id, CLOSE_DOC);
        w.borrow_mut().insert(SHOW_OPENED_DOC.id, SHOW_OPENED_DOC);
    });

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("editor."))
    {
        VIEW_SHORTCUT.with(|v| {
            VIEW_COMMAND_REGISTRY.with(|r| {
                if let Some(cmd) = r.borrow().get(command_id.as_str()) {
                    v.borrow_mut().insert(shortcut.clone(), *cmd);
                }
            });
        });
    }
    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("window."))
    {
        WINDOW_SHORTCUT.with(|v| {
            WINDOW_COMMAND_REGISTRY.with(|r| {
                if let Some(cmd) = r.borrow().get(command_id.as_str()) {
                    dbg!(command_id);
                    v.borrow_mut().insert(shortcut.clone(), *cmd);
                }
            });
        });
    }

    Application::new().window(move |_| app_view(), None).run();

    Ok(())
}
