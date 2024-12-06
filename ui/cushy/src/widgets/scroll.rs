//! A container that scrolls its contents on a virtual surface.
use std::collections::HashMap;

use cushy::figures::units::{Px, UPx};
use cushy::figures::{
    IntoSigned, IntoUnsigned, Point, Rect, Size, Zero,
};

use cushy::context::{EventContext, WidgetContext};
use cushy::value::{Destination, Dynamic, DynamicReader, Source};
use cushy::widget::{
    MakeWidget, MakeWidgetWithTag, WidgetId, WidgetInstance, WidgetRef,
    WidgetTag, WrapperWidget,
};
use cushy::widgets::Scroll;
use cushy::Lazy;

static SCROLLED_IDS: Lazy<Dynamic<HashMap<WidgetId, ScrollController>>> =
    Lazy::new(|| Dynamic::new(HashMap::new()));

#[derive(Debug)]
pub struct Scrollable {
    child: WidgetRef,
    pub controller: ScrollController,
}

impl Scrollable {
    pub fn new(child: impl MakeWidget, controller: ScrollController) -> Self {
        Self {
            child: child.make_widget().into_ref(),
            controller,
        }
    }
}

impl WrapperWidget for Scrollable {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }
}

pub trait WidgetScrollableExt {
    fn scrollable(self) -> Scrollable;
    fn scrollable_horizontally(self) -> Scrollable;
    fn scrollable_vertically(self) -> Scrollable;
}

impl WidgetScrollableExt for WidgetInstance {
    fn scrollable(self) -> Scrollable {
        let (tag, id) = WidgetTag::new();
        let s = Scroll::new(self);
        let scroller = ScrollController::new(
            s.scroll.clone(),
            s.control_size().clone(),
            s.max_scroll().clone(),
            id,
        );

        SCROLLED_IDS.lock().insert(id, scroller.clone());
        Scrollable::new(s.make_with_tag(tag), scroller)
    }
    fn scrollable_horizontally(self) -> Scrollable {
        let (tag, id) = WidgetTag::new();
        let s = Scroll::horizontal(self);
        let scroller = ScrollController::new(
            s.scroll.clone(),
            s.control_size().clone(),
            s.max_scroll().clone(),
            id,
        );

        SCROLLED_IDS.lock().insert(id, scroller.clone());
        Scrollable::new(s.make_with_tag(tag), scroller)
    }
    fn scrollable_vertically(self) -> Scrollable {
        let (tag, id) = WidgetTag::new();
        let s = Scroll::vertical(self);
        let scroller = ScrollController::new(
            s.scroll.clone(),
            s.control_size().clone(),
            s.max_scroll().clone(),
            id,
        );

        SCROLLED_IDS.lock().insert(id, scroller.clone());
        Scrollable::new(s.make_with_tag(tag), scroller)
    }
}

#[derive(Debug, Clone)]
pub struct ScrollController {
    scroll: Dynamic<Point<UPx>>,
    control_size: DynamicReader<Size<UPx>>,
    max_scroll: DynamicReader<Point<UPx>>,
    scroll_id: WidgetId,
}
#[allow(dead_code)]
impl ScrollController {
    pub fn new(
        scroll: Dynamic<Point<UPx>>,
        control_size: DynamicReader<Size<UPx>>,
        max_scroll: DynamicReader<Point<UPx>>,
        scroll_id: WidgetId,
    ) -> Self {
        Self {
            scroll: scroll.clone(),
            control_size: control_size.clone(),
            max_scroll: max_scroll.clone(),
            scroll_id,
        }
    }
    pub fn make_region_visible(&mut self, region: Rect<Px>) {
        let viewport = Rect::new(
            self.scroll.get().into_signed(),
            self.control_size.get().into_signed(),
        );

        if viewport.contains(region.origin) && viewport.contains(region.origin + region.size) {
            return;
        }

        let x = if region.origin.x <= viewport.origin.x {
            region.origin.x
        } else if region.extent().x >= viewport.extent().x {
            region.extent().x - viewport.size.width
        } else {
            self.scroll.get().x.into_signed()
        }
        .clamp(Px::ZERO, self.max_scroll.get().x.into_signed())
        .into_unsigned();

        let y = if region.origin.y <= viewport.origin.y {
            region.origin.y
        } else if region.extent().y >= viewport.extent().y {
            region.extent().y - viewport.size.height
        } else {
            self.scroll.get().y.into_signed()
        }
        .clamp(Px::ZERO, self.max_scroll.get().y.into_signed())
        .into_unsigned();

        self.scroll.replace(Point::new(x, y));
    }

    pub fn scroll_to(&mut self, scroll: Point<UPx>) {
        self.scroll.replace(scroll);
    }

    pub fn scroll(&self) -> Dynamic<Point<UPx>> {
        self.scroll.clone()
    }

    pub fn mouse_wheel(
        &mut self,
        device_id: cushy::window::DeviceId,
        delta: cushy::kludgine::app::winit::event::MouseScrollDelta,
        phase: cushy::kludgine::app::winit::event::TouchPhase,
        context: &mut EventContext<'_>,
    ) {
        context
            .for_other(&self.scroll_id)
            .unwrap()
            .mouse_wheel(device_id, delta, phase);
    }
}

#[allow(dead_code)]
pub trait ContextScroller {
    fn scroll_to(&self, scroll: Point<UPx>);
    fn scroll(&self) -> Dynamic<Point<UPx>>;
    fn make_region_visible(&self, region: Rect<Px>);
}

fn get_parent_scroller(context: &WidgetContext<'_>) -> Option<ScrollController> {
    let mut parent = context.widget().parent();
    while let Some(widget) = parent {
        if SCROLLED_IDS.get().contains_key(&widget.id()) {
            return Some(SCROLLED_IDS.get()[&widget.id()].clone());
        }

        parent = widget.parent();
    }
    None
}

impl ContextScroller for WidgetContext<'_> {
    fn scroll_to(&self, scroll: Point<UPx>) {
        if let Some(controller) = get_parent_scroller(self) {
            controller.scroll.replace(scroll);
        }
    }

    fn scroll(&self) -> Dynamic<Point<UPx>> {
        if let Some(controller) = get_parent_scroller(self) {
            controller.scroll.clone()
        } else {
            Dynamic::new(Point::default())
        }
    }

    fn make_region_visible(&self, region: Rect<Px>) {
        if let Some(mut controller) = get_parent_scroller(self) {
            controller.make_region_visible(region);
        }
    }
}
