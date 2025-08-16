use crate::host::{HostingToUIMessage, UIToHostingMessage};
use libadwaita::{
    glib::object::{IsA, ObjectExt},
    gtk::{
        Align, Button, Entry, EntryBuffer, Label, MediaFile, MediaStream, Stack, Widget,
        prelude::{BoxExt, ButtonExt, EditableExt, EditableExtManual, EntryBufferExtManual},
    },
};
use std::{
    rc::Rc,
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
};

#[derive(Debug, Default)]
struct HostState {
    message_sender: Rc<Mutex<Option<Sender<UIToHostingMessage>>>>,
    message_receiver: Mutex<Option<Receiver<HostingToUIMessage>>>,
}

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

    let host_button = Button::builder()
        .label("Host")
        .css_classes(["suggested-action"])
        .width_request(200)
        .halign(Align::Center)
        .build();

    let host_page = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .valign(Align::Center)
        .spacing(16)
        .build();
    host_page.append(&title);
    host_page.append(&port_box);
    host_page.append(&host_button);

    let title = Label::builder()
        .label("Hosting...")
        .css_classes(["title-1"])
        .build();

    let stop_button = Button::builder()
        .label("Stop")
        .css_classes(["destructive-action"])
        .width_request(200)
        .halign(Align::Center)
        .build();

    let hosting_page = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .valign(Align::Center)
        .spacing(16)
        .build();
    hosting_page.append(&title);
    hosting_page.append(&stop_button);

    let stack = Stack::new();
    stack.add_titled(&host_page, Some("host"), "Host");
    stack.add_titled(&hosting_page, Some("hosting"), "Hosting");

    let state = HostState::default();

    let stack_clone = stack.clone();
    let sender_clone = state.message_sender.clone();
    host_button.connect_clicked(move |_| {
        if port_buffer.text().len() < 4 {
            return;
        }
        let (sender, receiver) = start_hosting(port_buffer.text().to_string());
        *sender_clone.lock().unwrap() = Some(sender);
        *state.message_receiver.lock().unwrap() = Some(receiver);
        stack_clone.set_visible_child(&hosting_page);
    });

    let stack_clone = stack.clone();
    let sender_clone = state.message_sender.clone();
    stop_button.connect_clicked(move |_| {
        sender_clone
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .send(UIToHostingMessage::Stop)
            .unwrap();
        stack_clone.set_visible_child(&host_page);
    });

    stack
}

fn start_hosting(
    port_string: String,
) -> (Sender<UIToHostingMessage>, Receiver<HostingToUIMessage>) {
    let port: u16 = port_string.parse().unwrap();
    println!("Hosting on port {}", port_string);

    let (sender0, receiver0) = mpsc::channel::<HostingToUIMessage>();
    let (sender1, receiver1) = mpsc::channel::<UIToHostingMessage>();

    std::thread::spawn(move || crate::host::host(port, sender0, receiver1));
    (sender1, receiver0)
}
