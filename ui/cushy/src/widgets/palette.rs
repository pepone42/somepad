use std::sync::Arc;

use cushy::context::EventContext;
use cushy::figures::units::{Lp, Px};
use cushy::figures::{Point, Rect, ScreenScale, Size, Zero};
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};

use cushy::value::{Dynamic, Source};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, WidgetId, WidgetRef, WidgetTag, WrapperWidget,
    HANDLED, IGNORED,
};

use cushy::context;
use cushy::widgets::layers::Modal;
use cushy::widgets::scroll::ScrollBarThickness;
use cushy::widgets::{Custom, Scroll};
use cushy::window::KeyEvent;
use ndoc::Document;

use crate::shortcut::{event_match, Shortcut};

use super::filtered_list::{Filter, FilteredList};
use super::scroll::ContextScroller;
use super::scroll::WidgetScrollableExt;
use super::text_editor::TextEditor;

#[derive(Clone)]
pub struct Palette {
    child: WidgetRef,
    action: PaletteAction,
    input: Dynamic<Document>,
    has_items: bool,
    filter: Dynamic<Filter>,
    filter_id: WidgetId,
    next_key: Option<Shortcut>,
    prev_key: Option<Shortcut>,
    owner_id: WidgetId,
    modal: Modal,
}

impl Palette {
    pub fn new(state: PaletteState) -> Self {
        let input = Dynamic::new(Document::default());
        let str_input = input.map_each(|d| d.rope.to_string());
        let selected_idx = state.selected_idx;
        let action = state.action.clone();
        let (filter_tag, filter_id) = WidgetTag::new();
        let has_items = state.items.is_some();
        let filtered_list = if let Some(items) = state.items {
            FilteredList::new(
                items.clone(),
                str_input.clone(),
                selected_idx,
                action.clone(),
                state.owner,
                state.modal.clone(),
            )
        } else {
            FilteredList::new(
                Vec::new(),
                str_input.clone(),
                selected_idx,
                action.clone(),
                state.owner,
                state.modal.clone(),
            )
        };

        let filter = filtered_list.filter.clone();
        let pal: cushy::widgets::Align = Custom::new(
            state
                .description
                .clone()
                .and(
                    Custom::new(
                        TextEditor::as_input(input.clone())
                            .make_widget()
                            .scrollable_horizontally()
                            .with(&ScrollBarThickness, Lp::points(0)),
                    )
                    .on_mounted(move |c| c.focus()),
                )
                .and(
                    filtered_list.width(Lp::new(550))
                        .make_with_tag(filter_tag)
                        .scrollable_vertically()
                        .expand_vertically()
                        .height(Lp::new(500))
                        ,
                )
                .into_rows()
                .width(Lp::new(550))
                //.height(Lp::new(250)..Lp::new(500)), //Lp::ZERO..)
        )
        .on_redraw(|c| {
            let bg_color = c.get(&cushy::styles::components::SurfaceColor);
            c.gfx.fill(bg_color);
        })
        .on_hit_test(|_, _| true)
        .on_mouse_down(|_, _, _, _| HANDLED)
        .centered()
        .align_top();

        Palette {
            child: pal.make_widget().into_ref(),
            action,
            input,
            has_items,
            filter,
            filter_id,
            next_key: state.next_key,
            prev_key: state.prev_key,
            owner_id: state.owner,
            modal: state.modal,
        }
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

    fn close_palette(&self, context: &mut EventContext) {
        self.modal.dismiss();
        context.for_other(&self.owner_id).unwrap().focus();
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
            if let Some(s) = &self.next_key {
                if matches!(input.logical_key, Key::Named(NamedKey::Control))
                    && s.modifiers.control_key()
                {
                    self.close_palette(context);
                    if self.has_items {
                        let item = self.filter.get().selected_item.get();
                        if let Some(idx) = item {
                            (self.action)(
                                &mut context.for_other(&self.owner_id).unwrap(),
                                idx.index,
                                idx.text,
                            );
                        }
                    }
                }
            }
            return IGNORED;
        }

        if let Some(s) = &self.next_key {
            if event_match(&input, context.modifiers(), s.clone()) {
                self.filter.lock().next();
                self.scroll_to(context);
                return HANDLED;
            }
        }
        if let Some(s) = &self.prev_key {
            if event_match(&input, context.modifiers(), s.clone()) {
                self.filter.lock().prev();
                self.scroll_to(context);
                return HANDLED;
            }
        }
        match input.logical_key {
            Key::Named(NamedKey::Enter) => {
                self.close_palette(context);
                if self.has_items {
                    let item = self.filter.get().selected_item.get();
                    if let Some(idx) = item {
                        (self.action)(
                            &mut context.for_other(&self.owner_id).unwrap(),
                            idx.index,
                            idx.text,
                        );
                    }
                } else {
                    (self.action)(
                        &mut context.for_other(&self.owner_id).unwrap(),
                        0,
                        self.input.get().rope.to_string(),
                    );
                }

                HANDLED
            }
            Key::Named(NamedKey::Escape) => {
                self.close_palette(context);
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
        self.close_palette(_context);
        HANDLED
    }
}

pub(super) type PaletteAction =
    Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>;

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
    modal: Modal,
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
    pub fn new(modal: Modal) -> Self {
        PaletteState {
            description: String::default(),
            action: Arc::new(|_, _, _| ()),
            owner: WidgetTag::unique().id(),
            active: false,
            prev_key: None,
            next_key: None,
            selected_idx: 0,
            items: None,
            modal,
        }
    }
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn description(mut self, description: &'static str) -> Self {
        self.description = description.into();
        self
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

    pub fn show(self) {
        let modal = self.modal.clone();
        modal.present(Palette::new(self));
    }
    pub fn owner(mut self, owner: WidgetId) -> Self {
        self.owner = owner;
        self
    }
}
