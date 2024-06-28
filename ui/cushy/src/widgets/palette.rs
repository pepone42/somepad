use std::sync::Arc;

use cushy::context::{AsEventContext, EventContext};
use cushy::figures::units::{Lp, Px};
use cushy::figures::Zero;
use cushy::kludgine::app::winit::event::ElementState;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};

use cushy::styles::components::FontFamily;
use cushy::value::{Dynamic, Source};
use cushy::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetRef, WidgetTag, WrapperWidget, HANDLED, IGNORED
};

use cushy::widgets::{Custom, Input};
use cushy::window::KeyEvent;
use cushy::{context, Lazy};
use ndoc::Document;

use super::filtered_list::{self, Filter, FilteredList};
use super::scroll::{MyScroll, ScrollController};
use super::text_editor::TextEditor;

#[derive(PartialEq, Eq, Clone)]
pub struct Palette {
    description: Dynamic<String>,
    child: WidgetRef,
    action: Dynamic<Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>>,
    input: Dynamic<Document>,
    items: Option<Vec<String>>,
    // filtered_item_idx: Dynamic<Option<usize>>,
    filter: Dynamic<Filter>,
    filter_id: WidgetId,
}

impl Palette {
    pub fn new() -> Self {
        let input = Dynamic::new(Document::default());
        let str_input = input.map_each(|d| dbg!(d.rope.to_string()));
        let (filter_tag, filter_id) = WidgetTag::new();
        let filtered_list = if let Some(items) = PALETTE_STATE.get().items {
            FilteredList::new(items.clone(), str_input.clone())
        } else {
            FilteredList::new(Vec::new(), str_input.clone())
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
                        ).pad()
                        ,
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
        device_id: cushy::window::DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut context::EventContext<'_>,
    ) -> EventHandling {
        if input.state == ElementState::Released {
            return IGNORED;
        }
        //context.redraw_when_changed(&self.filtered_item_idx);
        match input.logical_key {
            Key::Named(NamedKey::Enter) => {
                if let Some(items) = &self.items {
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
                PALETTE_STATE.lock().active = false;

                HANDLED
            }
            Key::Named(NamedKey::Escape) => {
                PALETTE_STATE.lock().active = false;
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
            _ => IGNORED,
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

#[derive(Clone)]
pub(super) struct PaletteState {
    description: String,
    action: Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>,
    owner: WidgetId,
    active: bool,
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
            items: None,
        }
    }
    pub fn active(&self) -> bool {
        self.active
    }
}

pub(super) static PALETTE_STATE: Lazy<Dynamic<PaletteState>> =
    Lazy::new(|| Dynamic::new(PaletteState::new()));

pub fn ask<F: Fn(&mut EventContext, usize, String) + 'static + Send + Sync>(
    owner: WidgetId,
    description: &str,
    action: F,
) {
    let mut p = PALETTE_STATE.lock();
    p.description = description.to_string();
    p.action = Arc::new(action);
    p.owner = owner;
    p.active = true;
    p.items = None;
}

pub fn choose(
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
    p.items = Some(items);
}
