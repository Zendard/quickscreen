use libadwaita::{
    HeaderBar, ToolbarView, ViewStack, ViewSwitcher,
    gtk::{Orientation, prelude::BoxExt},
};

mod host;
mod join;

pub fn build_home() -> libadwaita::gtk::Box {
    let join_page = join::build_page();
    let host_page = host::build_page();

    let view_stack = ViewStack::new();
    view_stack.add_titled_with_icon(
        &join_page,
        Some("join"),
        "Join",
        "network-wireless-hotspot-symbolic",
    );
    view_stack.add_titled_with_icon(&host_page, Some("host"), "Host", "screen-shared");

    let view_switcher = ViewSwitcher::builder()
        .stack(&view_stack)
        .policy(libadwaita::ViewSwitcherPolicy::Wide)
        .build();

    let header_bar = HeaderBar::builder().title_widget(&view_switcher).build();

    let toolbar_view = ToolbarView::new();
    toolbar_view.set_content(Some(&view_stack));
    toolbar_view.add_top_bar(&header_bar);

    let content = libadwaita::gtk::Box::new(Orientation::Vertical, 0);
    content.append(&header_bar);
    content.append(&toolbar_view);
    content
}
