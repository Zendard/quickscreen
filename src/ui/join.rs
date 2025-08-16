use libadwaita::{
    glib::object::IsA,
    gtk::{Label, Widget},
};

pub fn build_page() -> impl IsA<Widget> {
    Label::new(Some("Join"))
}
