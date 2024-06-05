use std::sync::Arc;

use cushy::context::EventContext;
use cushy::figures::units::{Lp, Px};
use cushy::figures::Zero;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};

use cushy::value::{Dynamic, Source};
use cushy::widget::{
    EventHandling, MakeWidget, WidgetId, WidgetRef, WidgetTag, WrapperWidget, HANDLED, IGNORED,
};

use cushy::widgets::{Custom, Input};
use cushy::window::KeyEvent;
use cushy::{context, Lazy};

use super::filtered_list::{self, FilteredList};

#[derive(PartialEq, Eq, Clone)]
pub struct Palette {
    description: Dynamic<String>,
    child: WidgetRef,
    action: Dynamic<Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>>,
    input: Dynamic<String>,
    items: Option<Vec<String>>,
    selected_item: Dynamic<Option<usize>>,
}

impl Palette {
    pub fn new() -> Self {
        let input = Dynamic::new(String::default());
        let filtered_list =if let Some(items) = PALETTE_STATE.get().items {
            FilteredList::new(items.clone(), input.clone())
        } else {
            FilteredList::new(Vec::new(), input.clone())
        };
        let selected_item= filtered_list.selected_item.clone();
        let pal = Custom::new(
            PALETTE_STATE.get().description
                .clone()
                .and(Custom::new(Input::new(input.clone())).on_mounted(move |c| c.focus()))
                .and(filtered_list)
                .into_rows()
                .width(Lp::new(250))
                .height(Lp::ZERO..Lp::new(250)),
        )
        .on_redraw(|c| {
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
            selected_item
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

    fn keyboard_input(
        &mut self,
        _device_id: cushy::window::DeviceId,
        input: KeyEvent,
        _is_synthetic: bool,
        context: &mut context::EventContext<'_>,
    ) -> EventHandling {
        match input.logical_key {
            Key::Named(NamedKey::Enter) => {
                if let Some(items) = &self.items {
                    let idx = self.selected_item.get();
                    if let Some(idx) = idx {
                        self.action.get()(
                            &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                            idx,
                            items[idx].clone(),
                        );
                    }
                } else {
                    self.action.get()(
                        &mut context.for_other(&PALETTE_STATE.get().owner).unwrap(),
                        0,
                        self.input.get().clone(),
                    );
                }
                PALETTE_STATE.lock().active = false;

                HANDLED
            }
            Key::Named(NamedKey::Escape) => {
                PALETTE_STATE.lock().active = false;
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
        f.debug_struct("PaletteState").field("description", &self.description).field("action", &"Skipped").field("owner", &self.owner).field("active", &self.active).finish()
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

pub fn choose(owner: WidgetId, description: &str, items: Vec<String>, action: impl Fn(&mut EventContext, usize, String) + 'static + Send + Sync) {
    let mut p = PALETTE_STATE.lock();
    p.description = description.to_string();
    p.action = Arc::new(action);
    p.owner = owner;
    p.active = true;
    p.items = Some(items);
}
