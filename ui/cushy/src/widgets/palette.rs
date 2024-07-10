use std::sync::Arc;

use cushy::context::EventContext;
use cushy::figures::units::{Lp, Px};
use cushy::figures::{Point, Rect, ScreenScale, Size, Zero};
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};

use cushy::value::{Dynamic, Source, Switchable};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetRef, WidgetTag,
    WrapperWidget, HANDLED, IGNORED,
};

use cushy::widgets::Custom;
use cushy::window::KeyEvent;
use cushy::{context, Lazy};
use ndoc::Document;

use crate::shortcut::{event_match, Shortcut};

use super::filtered_list::{Filter, FilteredList};
use super::scroll::{ContextScroller, MyScroll};
use super::text_editor::TextEditor;

#[derive(PartialEq, Eq, Clone)]
pub struct Palette {
    description: Dynamic<String>,
    child: WidgetRef,
    action: Dynamic<PaletteAction>,
    input: Dynamic<Document>,
    items: Option<Vec<String>>,
    filter: Dynamic<Filter>,
    filter_id: WidgetId,
}

impl Palette {
    fn create() -> Self {
        let input = Dynamic::new(Document::default());
        let str_input = input.map_each(|d| d.rope.to_string());
        let selected_idx = PALETTE_STATE.get().selected_idx;
        let action = Dynamic::new(PALETTE_STATE.get().action.clone());
        let (filter_tag, filter_id) = WidgetTag::new();
        let filtered_list = if let Some(items) = PALETTE_STATE.get().items {
            FilteredList::new(items.clone(), str_input.clone(), selected_idx, action.clone())
        } else {
            FilteredList::new(Vec::new(), str_input.clone(), selected_idx, action.clone())
        };

        let filter = filtered_list.filter.clone();
        let pal: cushy::widgets::Align = Custom::new(
            PALETTE_STATE
                .get()
                .description
                .clone()
                .and(
                    Custom::new(MyScroll::horizontal(TextEditor::as_input(input.clone())))
                        .on_mounted(move |c| c.focus()),
                )
                .and(MyScroll::vertical(filtered_list.make_with_tag(filter_tag)).expand())
                .into_rows()
                .width(Lp::new(550))
                .height(Lp::ZERO..Lp::new(250)),
        )
        .on_redraw(|c| {
            c.apply_current_font_settings();

            c.gfx.set_font_family(cushy::styles::FamilyOwned::SansSerif);
            c.gfx.set_font_size(Px::new(12));
            let bg_color = c.get(&cushy::styles::components::SurfaceColor);
            c.gfx.fill(bg_color);
        })
        .on_hit_test(|_, _| true)
        .on_mouse_down(|_, _, _, _| HANDLED)
        .centered()
        .align_top();

        Palette {
            description: PALETTE_STATE.get().description.into(),
            child: pal.make_widget().widget_ref(),
            action,
            input,
            items: PALETTE_STATE.get().items,
            filter,
            filter_id,
        }
    }
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> impl Widget {
        let palette = PALETTE_STATE.map_each(|p| p.active());
        palette.switcher(move |current, _active| {
            if *current {
                Palette::create().make_widget()
            } else {
                Custom::empty().make_widget()
            }
        })
    }

    fn scroll_to(&mut self, context: &mut EventContext) {
        if let Some(idx) = self.filter.get().selected_idx.get() {
            let line_height = context
                .kludgine
                .line_height()
                .into_px(context.kludgine.scale());
            let y = line_height * Px::new(idx as i32);
            context
                .for_other(&self.filter_id)
                .unwrap()
                .make_region_visible(Rect::new(
                    Point::new(Px::ZERO, y - (line_height)),
                    Size::new(Px::ZERO, line_height * 4),
                ));
        }
    }
}

impl std::fmt::Debug for Palette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Palette")
            .field("child", &self.child)
            .field("action", &"(closure Fn Skipped)")
            .finish()
    }
}

impl WrapperWidget for Palette {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        if let Some(id) = context.widget().parent().and_then(|p| p.next_focus()) {
            context.for_other(&id).focus();
        }
    }

    fn adjust_child_constraints(
        &mut self,
        available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<cushy::ConstraintLimit> {
        context.invalidate_when_changed(&self.input);
        available_space
    }

    fn keyboard_input(
        &mut self,
        _device_id: cushy::window::DeviceId,
        input: KeyEvent,
        _is_synthetic: bool,
        context: &mut context::EventContext<'_>,
    ) -> EventHandling {
        if input.state == ElementState::Released {
            if let Some(s) = PALETTE_STATE.get().next_key {
                if matches!(input.logical_key, Key::Named(NamedKey::Control))
                    && s.modifiers.control_key()
                {
                    if self.items.is_some() {
                        let item = self.filter.get().selected_item.get();
                        if let Some(idx) = item {
                            self.action.get()(
                                &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                                idx.index,
                                idx.text,
                            );
                        }
                    }
                    close_palette();
                }
            }
            return IGNORED;
        }

        if let Some(s) = PALETTE_STATE.get().next_key {
            if event_match(&input, context.modifiers(), s) {
                self.filter.lock().next();
                self.scroll_to(context);
                return HANDLED;
            }
        }
        if let Some(s) = PALETTE_STATE.get().prev_key {
            if event_match(&input, context.modifiers(), s) {
                self.filter.lock().prev();
                self.scroll_to(context);
                return HANDLED;
            }
        }
        match input.logical_key {
            Key::Named(NamedKey::Enter) => {
                if self.items.is_some() {
                    let item = self.filter.get().selected_item.get();
                    if let Some(idx) = item {
                        self.action.get()(
                            &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                            idx.index,
                            idx.text,
                        );
                    }
                } else {
                    self.action.get()(
                        &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                        0,
                        self.input.get().rope.to_string(),
                    );
                }
                close_palette();

                HANDLED
            }
            Key::Named(NamedKey::Escape) => {
                close_palette();
                HANDLED
            }
            Key::Named(NamedKey::ArrowDown) => {
                self.filter.lock().next();
                self.scroll_to(context);
                HANDLED
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.filter.lock().prev();
                self.scroll_to(context);
                HANDLED
            }
            _ => HANDLED,
        }
    }

    fn hit_test(
        &mut self,
        _location: cushy::figures::Point<Px>,
        _context: &mut EventContext<'_>,
    ) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        _location: cushy::figures::Point<Px>,
        _device_id: cushy::window::DeviceId,
        _button: cushy::kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_>,
    ) -> EventHandling {
        PALETTE_STATE.lock().active = false;
        HANDLED
    }
}

pub(super) type PaletteAction = Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>;

#[derive(Clone)]
pub struct PaletteState {
    description: String,
    action: PaletteAction,
    pub owner: WidgetId,
    active: bool,
    next_key: Option<Shortcut>,
    prev_key: Option<Shortcut>,
    selected_idx: usize,
    items: Option<Vec<String>>,
}

impl std::fmt::Debug for PaletteState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaletteState")
            .field("description", &self.description)
            .field("action", &"Skipped")
            .field("owner", &self.owner)
            .field("active", &self.active)
            .finish()
    }
}

impl PaletteState {
    fn new() -> Self {
        PaletteState {
            description: String::default(),
            action: Arc::new(|_, _, _| ()),
            owner: WidgetTag::unique().id(),
            active: false,
            prev_key: None,
            next_key: None,
            selected_idx: 0,
            items: None,
        }
    }
    pub fn active(&self) -> bool {
        self.active
    }

    pub fn accept<F: Fn(&mut EventContext, usize, String) + 'static + Send + Sync>(
        mut self,
        action: F,
    ) -> Self {
        self.action = Arc::new(action);
        self
    }

    pub fn next_key(mut self, next_key: Shortcut) -> Self {
        self.next_key = Some(next_key);
        self
    }

    pub fn prev_key(mut self, prev_key: Shortcut) -> Self {
        self.prev_key = Some(prev_key);
        self
    }

    pub fn items(mut self, items: Vec<String>) -> Self {
        self.items = Some(items);
        self
    }

    pub fn selected_idx(mut self, selected_idx: usize) -> Self {
        self.selected_idx = selected_idx;
        self
    }

    pub fn show(mut self) {
        self.active = true;
        *PALETTE_STATE.lock() = self;
    }
    pub fn owner(mut self, owner: WidgetId) -> Self {
        self.owner = owner;
        self
    }
}

pub (super) static PALETTE_STATE: Lazy<Dynamic<PaletteState>> = Lazy::new(|| Dynamic::new(PaletteState::new()));

pub (super) fn close_palette() {
    PALETTE_STATE.lock().active = false;
}

pub fn palette(description: &str) -> PaletteState {
    PaletteState {
        description: description.to_string(),
        ..PaletteState::new()
    }
}

pub trait PaletteExt {
    fn palette(&self, description: &str) -> PaletteState;
}

impl<'a> PaletteExt for EventContext<'a> {
    fn palette(&self, description: &str) -> PaletteState {
        palette(description).owner(self.widget().id())
    }
}
