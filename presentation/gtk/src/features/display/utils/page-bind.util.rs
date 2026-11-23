use std::rc::{Rc, Weak};

/// Wraps a `Weak` page handle so action callbacks upgrade before dispatch.
pub fn bind<T: 'static, P: 'static>(this: &Weak<P>, run: fn(&Rc<P>, T)) -> Rc<dyn Fn(T)> {
    let this = this.clone();
    Rc::new(move |value| {
        if let Some(page) = this.upgrade() {
            run(&page, value);
        }
    })
}
