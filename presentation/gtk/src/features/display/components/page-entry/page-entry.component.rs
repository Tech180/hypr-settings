use std::rc::{Rc, Weak};

use crate::features::settings_shell::interfaces::{Section, ShellContext};

use crate::features::display::components::page_state::Page;

pub fn build_section(ctx: &ShellContext) -> Rc<dyn Section> {
    let page = Rc::new_cyclic(|weak: &Weak<Page>| Page::new(ctx.clone(), weak));
    page.reload();
    page
}
