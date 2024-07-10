use cushy::context::{AsEventContext, EventContext, Trackable};
use cushy::figures::units::{Lp, Px, UPx};
use cushy::figures::{Point, Rect, ScreenScale, Size, Zero};
use cushy::styles::components;
use cushy::value::{Dynamic, IntoValue, Source, Value};
use cushy::widget::{MakeWidget, MakeWidgetList, MountedChildren, Widget, WidgetList, WidgetRef};
use cushy::widgets::grid::Orientation;
use cushy::ConstraintLimit;

#[derive(Debug)]
pub struct Splitter {
    orientation: Orientation,
    width: Dynamic<UPx>,
    children: Value<WidgetList>,
    mounted: MountedChildren,
}

impl Splitter {
    pub fn new(left: impl MakeWidget, right: impl MakeWidget, width: Dynamic<UPx>) -> Self {
        let mut children = WidgetList::new();
        children.push(left);
        children.push(ResizeHandle::new(width.clone()));
        children.push(right);
        Splitter {
            orientation: Orientation::Row,
            width,
            children: children.into_value(),
            mounted: MountedChildren::default(),
        }
    }
    fn synchronize_children(&mut self, context: &mut EventContext<'_>) {
        self.children.invalidate_when_changed(context);
        self.mounted.synchronize_with(&self.children, context);
    }
}

impl Widget for Splitter {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        self.synchronize_children(&mut context.as_event_context());
        for children in self.mounted.children() {
            context.for_other(children).redraw();
        }
    }

    fn layout(
        &mut self,
        available_space: cushy::figures::Size<cushy::ConstraintLimit>,
        context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
    ) -> cushy::figures::Size<UPx> {
        self.synchronize_children(&mut context.as_event_context());
        self.children.invalidate_when_changed(context);
        let scale = context.gfx.scale();
        let width = self.width.get();

        let mut height = available_space.height;
        let mut x = 0.0;
        let mut y = 0.0;

        let left = self.mounted.children().first().unwrap();
        let handle = self.mounted.children().get(1).unwrap();
        let right = self.mounted.children().get(2).unwrap();
        let size = if self.orientation == Orientation::Row {
            let l = context.for_other(left).layout(Size::new(ConstraintLimit::Fill(width), available_space.height));
            let h = context.for_other(handle).layout(Size::new(ConstraintLimit::Fill(UPx::new(5)), available_space.height));
            let r =   context.for_other(right).layout(Size::new(available_space.width - width - UPx::new(5), available_space.height));

            context.set_child_layout(left, Rect::new(Point::ZERO, l.into_px(scale)));
            context.set_child_layout(handle, Rect::new(Point::new(l.width.into_px(scale), Px::ZERO), l.into_px(scale)));
            context.set_child_layout(right, Rect::new(Point::new((l.width+h.width).into_px(scale), Px::ZERO),r.into_px(scale)));

            Size::new(l.width + r.width + 5, l.height.max(r.height))
        } else {
            let l = context.for_other(left).layout(Size::new(available_space.width,ConstraintLimit::Fill(width)));
            let r = context.for_other(right).layout(Size::new(available_space.height, available_space.width - width));
            dbg!(Size::new(l.width.max(r.width), l.height + r.height))
        };

        size
    }
}


#[derive(Debug)]
pub struct ResizeHandle {
    width: Dynamic<UPx>,
    hovered: Dynamic<bool>,
    dragged: Dynamic<bool>,
}

impl ResizeHandle {
    pub fn new(width: Dynamic<UPx>) -> Self {
        ResizeHandle { width, hovered: Dynamic::new(false), dragged: Dynamic::new(false)}
    }
}

impl Widget for ResizeHandle {
    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        if self.hovered.get() || self.dragged.get() {
            context.fill(context.get(&components::DefaultHoveredBackgroundColor));
        } else {
            context.fill(context.get(&components::WidgetBackground));
        }
    }
}