use cushy::widget::WidgetInstance;


pub trait DowncastWidget<T,R,F: FnMut(&T) -> R> {
    fn use_as(&self, action: F) -> R;
}

impl<T: 'static,R, F: FnMut(&T) -> R> DowncastWidget<T,R,F> for WidgetInstance {
    fn use_as(&self, mut action: F) -> R{
        let guard = self.lock();
        let instance = guard.downcast_ref::<T>().unwrap();
        action(instance)
    }
}