
use std::sync::Arc;

use cushy::context::EventContext;
use cushy::figures::units::Px;
use cushy::figures::Size;
use cushy::kludgine::app::winit::keyboard::{Key, NamedKey};
use cushy::styles::Color;
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{
    EventHandling, MakeWidget, WidgetId, WidgetRef, WrapperWidget, HANDLED, IGNORED,
};

use cushy::widgets::{Custom, Input};
use cushy::window::KeyEvent;
use cushy::{context, Lazy};

#[derive(PartialEq, Eq, Clone)]
pub struct Palette {
    child: WidgetRef,
    action: Dynamic<Option<Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>>>,
    input: Dynamic<String>,
}

impl Palette {
    pub fn new() -> Self {
        let input = Dynamic::new(String::default());
        let pal = "palette"
            .and(Custom::new(Input::new(input.clone())).on_mounted(move |c| c.focus()))
            .into_rows().size(Size::new(Px::new(200), Px::new(200))).background_color(Color::new(0x34, 0x3D, 0x46, 0xFF))
            .centered()
            .align_top();

        //let child = Custom::empty().size(Size::new(width, height))

        Palette {
            child: pal.make_widget().widget_ref(),
            action: PALETTE_ACTION.clone(),
            input,
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
                self.action.get().unwrap()(
                    &mut context.for_other(&PALETTE_OWNER.get().unwrap()).unwrap(),
                    0,
                    self.input.get().clone(),
                );
                PALETTE.set(false);

                HANDLED
            }
            _ => IGNORED,
        }
    }
}

pub static PALETTE: Lazy<Dynamic<bool>> = Lazy::new(|| Dynamic::new(false));
static PALETTE_ACTION: Lazy<
    Dynamic<Option<Arc<dyn Fn(&mut EventContext, usize, String) + 'static + Send + Sync>>>,
> = Lazy::new(|| Dynamic::new(None));
static PALETTE_OWNER: Lazy<Dynamic<Option<WidgetId>>> = Lazy::new(|| Dynamic::new(None));

pub fn ask<F: Fn(&mut EventContext, usize, String) + 'static + Send + Sync>(
    owner: WidgetId,
    action: F,
) {
    *PALETTE_ACTION.lock() = Some(Arc::new(action));
    PALETTE.set(true);
    PALETTE_OWNER.set(Some(owner));
}
