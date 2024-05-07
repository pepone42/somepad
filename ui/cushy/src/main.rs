#[macro_use]
mod shortcut;
mod scroll;
mod settings;

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default;
use std::ops::Deref;
use std::os::raw;
use std::sync::{Arc, Mutex};

use cushy::figures::units::{self, Lp, Px, UPx};
use cushy::figures::{
    Abs, FloatConversion, IntoSigned, Point, Px2D, Rect, Roots, Round, ScreenScale, Size, Zero,
};
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};
use cushy::kludgine::cosmic_text::fontdb::ID;
use cushy::kludgine::cosmic_text::{Attrs, Buffer, Cursor, Family, FontSystem, LayoutRun, Metrics};
use cushy::kludgine::image::buffer;
use cushy::kludgine::shapes::{Path, PathBuilder, Shape};
use cushy::kludgine::text::{MeasuredText, Text};
use cushy::kludgine::{Drawable, DrawableExt};
use cushy::styles::components::CornerRadius;
use cushy::styles::{Color, CornerRadii, Dimension, FamilyOwned};
use cushy::value::{Destination, Dynamic, IntoReadOnly, Source};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance, WidgetRef, WidgetTag,
    WrapperWidget, HANDLED, IGNORED,
};
use cushy::widgets::{color, Custom, Data, Resize};
use cushy::window::KeyEvent;
use cushy::{context, Lazy, ModifiersExt, Run};
use ndoc::{rope_utils, Document};
use scroll::{MyScroll, ScrollController};
use settings::Settings;
use shortcut::{event_match, Shortcut};
use smol_str::SmolStr;

#[derive(Debug)]
pub struct TextEditor {
    doc: Dynamic<ndoc::Document>,
    viewport: Dynamic<Rect<Px>>,
    scroll_controller: Dynamic<ScrollController>,
}

impl TextEditor {
    fn point_to_grapheme(
        &self,
        line: usize,
        point: Point<Px>,
        font_system: &mut FontSystem,
    ) -> usize {
        // TODO: tab support
        let raw_text = self.doc.get().rope.line(line).to_string();
        //cushy::kludgine::cosmic_text::Buffer::new(font_system, Metrics::new(font_size, line_height))
        let mut buffer = Buffer::new(font_system, Metrics::new(15.0, 20.0));
        buffer.set_size(font_system, 1000., 1000.);
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
        rope_utils::byte_to_grapheme(&self.doc.get().rope.line(line as _), byte_idx)
    }

    fn grapheme_to_point(&self, line: usize, index: usize, font_system: &mut FontSystem) -> Px {
        // TODO: tab support
        let raw_text = self.doc.get().rope.line(line).to_string();
        //cushy::kludgine::cosmic_text::Buffer::new(font_system, Metrics::new(font_size, line_height))
        let mut buffer = Buffer::new(font_system, Metrics::new(15.0, 20.0));
        buffer.set_size(font_system, 1000., 1000.);
        buffer.set_text(
            font_system,
            &raw_text,
            Attrs::new().family(Family::Monospace),
            cushy::kludgine::cosmic_text::Shaping::Advanced,
        );
        let col = rope_utils::grapheme_to_byte(&self.doc.get().rope.line(line), index);
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

    fn refocus_main_selection(&self, font_system: &mut FontSystem) {
        if self.doc.get().selections.len() == 1 {
            let main_selection_head_x = self.grapheme_to_point(
                self.doc.get().selections[0].head.line,
                self.doc.get().selections[0].head.column,
                font_system,
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
        let translation = context.gfx.translation();
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
            let attrs = Attrs::new().family(Family::Monospace); //.color(cushy::kludgine::cosmic_text::Color::rgba(0xff,0xff,0xff,0xdd));

            context.gfx.set_text_attributes(attrs);

            if let Some(sl) = doc.get_style_line_info(i as _) {
                let mut buffer = Buffer::new(context.gfx.font_system(), Metrics::new(15.0, 20.0));
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
                    context.gfx.font_system(),
                    spans,
                    attrs,
                    cushy::kludgine::cosmic_text::Shaping::Advanced,
                );
                buffer.set_size(context.gfx.font_system(), 1000., 1000.);

                // context.gfx.draw_text(
                //     Text::new(&i.to_string(), Color::WHITE).translate_by(Point::new(-translation.x, y)),
                // );

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
            //dbg!(s.start(), s.end());
            let head = self
                .grapheme_to_point(s.head.line, s.head.column, context.gfx.font_system())
                .floor();
            //dbg!(start, end);

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

        // for i in first_line.get()..last_line.get() {
        //     let y = units::Px::new(i as _) * 20;
        //     let slice = doc.rope.slice(..);
        //     let raw_text = ndoc::rope_utils::get_line_info(&slice, i as _, 4);
        //     let attrs = Attrs::new().family(Family::Monospace); //.color(cushy::kludgine::cosmic_text::Color::rgba(0xff,0xff,0xff,0xdd));

        //     context.gfx.set_text_attributes(attrs);

        //     if let Some(sl) = doc.get_style_line_info(i as _) {
        //         let mut x = units::Px::ZERO;
        //         for s in sl.iter() {
        //             let t = &raw_text[s.range.start..s.range.end];
        //             let col = Color::new(
        //                 s.style.foreground.r,
        //                 s.style.foreground.g,
        //                 s.style.foreground.b,
        //                 s.style.foreground.a,
        //             );
        //             let t = Text::<units::Px>::new(t, col);
        //             let m = context.gfx.measure_text(t);

        //             context.gfx.draw_measured_text(
        //                 m.translate_by(Point::new(x, y)),
        //                 cushy::kludgine::text::TextOrigin::TopLeft,
        //             );
        //             x += m.size.width;
        //             if i == 1 {
        //                 dbg!(
        //                     m.size.width,
        //                     m.glyphs
        //                         .iter()
        //                         .fold(Px::ZERO, |acc, g| acc + g.info.line_width)
        //                 );
        //             }
        //         }
        //     }
        // }
    }

    fn layout(
        &mut self,
        available_space: Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let doc = self.doc.get();
        let height = doc.rope.len_lines() as f32 * 20.0;

        //dbg!(context.gfx.translation());
        self.viewport.set(Rect::new(
            context.gfx.translation().abs(),
            context.gfx.size().into_px(context.gfx.scale()),
        ));

        Size::new(UPx::new(1000), UPx::new(height.ceil() as _))
    }

    fn accept_focus(&mut self, context: &mut cushy::context::EventContext<'_>) -> bool {
        true
    }

    fn hit_test(
        &mut self,
        location: Point<units::Px>,
        context: &mut cushy::context::EventContext<'_>,
    ) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        location: Point<units::Px>,
        device_id: cushy::window::DeviceId,
        button: cushy::kludgine::app::winit::event::MouseButton,
        context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        context.focus();

        let line = ((self.viewport.get().origin.y + location.y) / 20)
            .floor()
            .get();

        let char_idx = self.point_to_grapheme(
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
        device_id: cushy::window::DeviceId,
        input: cushy::window::KeyEvent,
        is_synthetic: bool,
        context: &mut cushy::context::EventContext<'_>,
    ) -> cushy::widget::EventHandling {

        if input.state == ElementState::Pressed && context.modifiers().possible_shortcut() {
            let v = VIEW_SHORTCUT.lock().unwrap();
            for (shortcut, cmd) in v.iter() {
                if event_match(&input, context.modifiers(), shortcut.clone()) {
                    (cmd.action)(self);
                    return HANDLED;
                }
            }
        }

        if input.state == ElementState::Pressed && matches!(input.logical_key, Key::Named(_)) {
            match input.logical_key {
                Key::Named(NamedKey::Backspace) => {
                    self.doc.lock().backspace();
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::Delete) => {
                    self.doc.lock().delete();
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowLeft) if context.modifiers().word_select() => {
                    self.doc.lock().move_selections_word(
                        ndoc::MoveDirection::Left,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowRight) if context.modifiers().word_select() => {
                    self.doc.lock().move_selections_word(
                        ndoc::MoveDirection::Right,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    self.doc.lock().move_selections(
                        ndoc::MoveDirection::Left,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowRight) => {
                    self.doc.lock().move_selections(
                        ndoc::MoveDirection::Right,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowUp) => {
                    self.doc.lock().move_selections(ndoc::MoveDirection::Up, context.modifiers().only_shift());
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::ArrowDown) => {
                    self.doc.lock().move_selections(
                        ndoc::MoveDirection::Down,
                        context.modifiers().only_shift(),
                    );
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                Key::Named(NamedKey::Enter) => {
                    let mut doc = self.doc.lock();
                    let linefeed = doc.file_info.linefeed.to_string();
                    doc.insert(&linefeed);
                    self.refocus_main_selection(context.kludgine.font_system());
                    return HANDLED;
                }
                _ => {}
            }
        }

        match input.text {
            Some(t) if !context.modifiers().possible_shortcut() => {
                self.doc.lock().insert(&t);
                self.refocus_main_selection(context.kludgine.font_system());

                HANDLED
            }
            _ => IGNORED,
        }
    }
}

#[derive(Debug)]
pub struct EditorWindow {
    child: WidgetRef,
    documents: Dynamic<Vec<Dynamic<Document>>>,
}

impl EditorWindow {
    #[must_use]
    pub fn new(child: impl MakeWidget) -> impl MakeWidget {
        EditorWindow {
            child: child.widget_ref(),
            documents: Dynamic::new(Vec::new()),
        }
    }

    pub fn add_new_doc(&self, doc: Dynamic<Document>) {
        self.documents.lock().push(doc);
    }
}

impl WrapperWidget for EditorWindow {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn keyboard_input(
        &mut self,
        device_id: cushy::window::DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut cushy::context::EventContext<'_>,
    ) -> EventHandling {
        if input.state == ElementState::Pressed && context.modifiers().possible_shortcut() {
            let v = WINDOW_SHORTCUT.lock().unwrap();
            for (shortcut, cmd) in v.iter() {
                if event_match(&input, context.modifiers(), shortcut.clone()) {
                    (cmd.action)(self);
                    return HANDLED;
                }
            }
            return IGNORED;
        }
        IGNORED
    }
}

#[derive(Clone, Copy)]
pub struct ViewCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(&TextEditor),
}

#[derive(Clone, Copy)]
pub struct WindowCommand {
    pub name: &'static str,
    pub id: &'static str,
    pub action: fn(&EditorWindow),
}

pub static VIEW_SHORTCUT: Lazy<Arc<Mutex<HashMap<Shortcut, ViewCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
pub static VIEW_COMMAND_REGISTRY: Lazy<Arc<Mutex<HashMap<&'static str, ViewCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static WINDOW_SHORTCUT: Lazy<Arc<Mutex<HashMap<Shortcut, WindowCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));
pub static WINDOW_COMMAND_REGISTRY: Lazy<Arc<Mutex<HashMap<&'static str, WindowCommand>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

const NEW_DOC: WindowCommand = WindowCommand {
    name: "New Document",
    id: "window.newdoc",
    action: |w| {
        dbg!("New doc!");
        w.add_new_doc(Dynamic::new(Document::default()));
    },
};

pub static SETTINGS: Lazy<Arc<Mutex<Settings>>> =
    Lazy::new(|| Arc::new(Mutex::new(Settings::load())));

pub fn get_settings() -> Settings {
    SETTINGS.lock().unwrap().clone()
}

fn main() -> anyhow::Result<()> {
    WINDOW_COMMAND_REGISTRY
        .lock()
        .unwrap()
        .insert(NEW_DOC.id, NEW_DOC);

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("editor."))
    {
        let mut v = VIEW_SHORTCUT.lock().unwrap();
        let r = VIEW_COMMAND_REGISTRY.lock().unwrap();
        if let Some(cmd) = r.get(command_id.as_str()) {
            dbg!(command_id, shortcut);
            v.insert(shortcut.clone(), *cmd);
        }
    }

    for (command_id, shortcut) in get_settings()
        .shortcuts
        .iter()
        .filter(|(id, _)| id.starts_with("window."))
    {
        let mut v = WINDOW_SHORTCUT.lock().unwrap();
        let r = WINDOW_COMMAND_REGISTRY.lock().unwrap();
        if let Some(cmd) = r.get(command_id.as_str()) {
            dbg!(command_id, shortcut);
            v.insert(shortcut.clone(), *cmd);
        }
    }

    ndoc::Document::init_highlighter();
    let doc = if let Some(path) = std::env::args().nth(1) {
        ndoc::Document::from_file(path)?
    } else {
        ndoc::Document::default()
    };
    let scroll_controller = Dynamic::new(ScrollController::default());
    EditorWindow::new(MyScroll::new(
        TextEditor {
            doc: Dynamic::new(doc),
            viewport: Dynamic::new(Rect::default()),
            scroll_controller: scroll_controller.clone(),
        }
        .with(
            &CornerRadius,
            CornerRadii::from(Dimension::Lp(Lp::points(0))),
        ),
        scroll_controller,
    ))
    .expand()
    .run()?;

    Ok(())
}
