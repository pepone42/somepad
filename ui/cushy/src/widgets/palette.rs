use std::sync::Arc;

use cushy::context::EventContext;
use cushy::figures::units::{Lp, Px};
use cushy::figures::Zero;
use cushy::kludgine::app::winit::event::{ElementState, Modifiers};
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};

use cushy::kludgine::wgpu::naga::proc::NameKey;
use cushy::value::{Dynamic, Source, Switchable};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetRef, WidgetTag,
    WrapperWidget, HANDLED, IGNORED,
};

use cushy::widgets::{select, Custom};
use cushy::window::KeyEvent;
use cushy::{context, Lazy};
use ndoc::Document;

use crate::shortcut::{event_match, ModifiersCustomExt, Shortcut};

use super::filtered_list::{Filter, FilteredList};
use super::scroll::{MyScroll, ScrollController};
use super::text_editor::TextEditor;

#[derive(PartialEq, Eq, Clone)]
pub struct Palette {
    description: Dynamic<String>,
    child: WidgetRef,
    action: Dynamic<PaletteAction>,
    input: Dynamic<Document>,
    items: Option<Vec<String>>,
    // filtered_item_idx: Dynamic<Option<usize>>,
    filter: Dynamic<Filter>,
    filter_id: WidgetId,
}

impl Palette {
    fn create() -> Self {
        let input = Dynamic::new(Document::default());
        let str_input = input.map_each(|d| dbg!(d.rope.to_string()));
        let (filter_tag, filter_id) = WidgetTag::new();
        let selected_idx = PALETTE_STATE.get().selected_idx;
        let filtered_list = if let Some(items) = PALETTE_STATE.get().items {
            FilteredList::new(items.clone(), str_input.clone(),selected_idx)
        } else {
            FilteredList::new(Vec::new(), str_input.clone(),selected_idx)
        };

        let filter = filtered_list.filter.clone();
        let scroller = Dynamic::new(ScrollController::default());
        let pal = Custom::new(
            PALETTE_STATE
                .get()
                .description
                .clone()
                .and(
                    Custom::new(
                        MyScroll::horizontal(
                            TextEditor::as_input(input.clone()).with_scroller(scroller.clone()),
                            scroller.clone(),
                        )
                        .pad(),
                    )
                    .on_mounted(move |c| c.focus()),
                )
                .and(filtered_list.make_with_tag(filter_tag).pad())
                .into_rows()
                .width(Lp::new(250))
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
            action: Dynamic::new(PALETTE_STATE.get().action.clone()),
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

    fn ask<F: Fn(&mut EventContext, usize, String) + 'static + Send + Sync>(
        owner: WidgetId,
        description: &str,
        action: F,
    ) {
        let mut p = PALETTE_STATE.lock();
        p.description = description.to_string();
        p.action = Arc::new(action);
        p.owner = owner;
        p.active = true;
        p.modifiers = None;
        p.next_key = None;
        p.prev_key = None;
        p.items = None;
    }

    fn choose(
        owner: WidgetId,
        description: &str,
        items: Vec<String>,
        action: impl Fn(&mut EventContext, usize, String) + 'static + Send + Sync,
    ) {
        let mut p = PALETTE_STATE.lock();
        p.description = description.to_string();
        p.action = Arc::new(action);
        p.owner = owner;
        p.active = true;
        p.modifiers = None;
        p.next_key = None;
        p.prev_key = None;
        p.items = Some(items);
    }
    fn quick_choose(
        owner: WidgetId,
        description: &str,
        items: Vec<String>,
        modifiers: Modifiers,
        next_key: Shortcut,
        prev_key: Shortcut,
        selected_idx: usize,
        action: impl Fn(&mut EventContext, usize, String) + 'static + Send + Sync,
    ) {
        let mut p = PALETTE_STATE.lock();
        p.description = description.to_string();
        p.action = Arc::new(action);
        p.owner = owner;
        p.active = true;
        p.modifiers = Some(modifiers);
        p.next_key = Some(next_key);
        p.prev_key = Some(prev_key);
        p.selected_idx = selected_idx;
        p.items = Some(items);
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
            if let Some(m) = PALETTE_STATE.get().modifiers {
                //dbg!(input.logical_key);
                if matches!(input.logical_key, Key::Named(NamedKey::Control)) && m.ctrl() {
                    if self.items.is_some() {
                        let item = self.filter.get().selected_item.get();
                        if let Some(idx) = item {
                            self.action.get()(
                                &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                                dbg!(idx.index),
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
                return HANDLED;
            }
        }
        if let Some(s) = PALETTE_STATE.get().prev_key {
            if event_match(&input, context.modifiers(), s) {
                self.filter.lock().prev();
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
                            dbg!(idx.index),
                            idx.text,
                        );
                    }
                } else {
                    self.action.get()(
                        &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                        0,
                        dbg!(self.input.get().rope.to_string()),
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
                HANDLED
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.filter.lock().prev();
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

type PaletteAction = Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>;

#[derive(Clone)]
pub(super) struct PaletteState {
    description: String,
    action: PaletteAction,
    owner: WidgetId,
    active: bool,
    modifiers: Option<Modifiers>,
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
            modifiers: None,
            prev_key: None,
            next_key: None,
            selected_idx: 0,
            items: None,
        }
    }
    pub fn active(&self) -> bool {
        self.active
    }
}

static PALETTE_STATE: Lazy<Dynamic<PaletteState>> = Lazy::new(|| Dynamic::new(PaletteState::new()));

fn close_palette() {
    PALETTE_STATE.lock().active = false;
}

pub trait PaletteExt {
    fn ask<F: Fn(&mut EventContext, usize, String) + 'static + Send + Sync>(
        &mut self,
        description: &str,
        action: F,
    );
    fn choose(
        &mut self,
        description: &str,
        items: Vec<String>,
        action: impl Fn(&mut EventContext, usize, String) + 'static + Send + Sync,
    );
    fn quick_choose(
        &mut self,
        description: &str,
        items: Vec<String>,
        next_key: Shortcut,
        prev_key: Shortcut,
        selected_idx: usize,
        action: impl Fn(&mut EventContext, usize, String) + 'static + Send + Sync,
    );
}

impl<'a> PaletteExt for EventContext<'a> {
    fn ask<F: Fn(&mut EventContext, usize, String) + 'static + Send + Sync>(
        &mut self,
        description: &str,
        action: F,
    ) {
        Palette::ask(self.widget().id(), description, action);
    }

    fn choose(
        &mut self,
        description: &str,
        items: Vec<String>,
        action: impl Fn(&mut EventContext, usize, String) + 'static + Send + Sync,
    ) {
        Palette::choose(self.widget().id(), description, items, action);
    }
    fn quick_choose(
        &mut self,
        description: &str,
        items: Vec<String>,
        next_key: Shortcut,
        prev_key: Shortcut,
        selected_idx: usize,
        action: impl Fn(&mut EventContext, usize, String) + 'static + Send + Sync,
    ) {
        Palette::quick_choose(
            self.widget().id(),
            description,
            items,
            self.modifiers(),
            next_key,prev_key,
            selected_idx,
            action,
        );
    }
}
