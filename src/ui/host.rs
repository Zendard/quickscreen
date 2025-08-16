use libadwaita::{
    glib::object::{IsA, ObjectExt},
    gtk::{
        Align, Button, Entry, EntryBuffer, Label, Widget,
        prelude::{BoxExt, EditableExt, EditableExtManual},
    },
};

pub fn build_page() -> impl IsA<Widget> {
    let title = Label::builder()
        .label("Host")
        .css_classes(["title-1"])
        .build();

    let port_label = Label::builder().label("Port").halign(Align::Start).build();
    let port_buffer = EntryBuffer::new(None::<String>);
    let port_input = Entry::builder()
        .placeholder_text("1234")
        .input_purpose(libadwaita::gtk::InputPurpose::Digits)
        .max_length(4)
        .buffer(&port_buffer)
        .build();
    let port_box = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .spacing(4)
        .halign(Align::Center)
        .width_request(200)
        .build();
    port_box.append(&port_label);
    port_box.append(&port_input);

    // Only allow digits to be typed in port_input
    port_input
        .delegate()
        .unwrap()
        .connect_insert_text(|entry, text, _position| {
            if !text.chars().all(|c| c.is_ascii_digit()) {
                entry.stop_signal_emission_by_name("insert-text");
            }
        });

    let button = Button::builder()
        .label("Host")
        .css_classes(["suggested-action"])
        .width_request(200)
        .halign(Align::Center)
        .build();

    let content = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .valign(Align::Center)
        .spacing(16)
        .build();
    content.append(&title);
    content.append(&port_box);
    content.append(&button);

    content
}
