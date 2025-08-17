use libadwaita::{
    glib::object::{IsA, ObjectExt},
    gtk::{
        Align, Button, Entry, EntryBuffer, Label, Stack, Widget,
        prelude::{BoxExt, ButtonExt, EditableExt, EditableExtManual, EntryBufferExtManual},
    },
};
use std::{
    rc::Rc,
    str::FromStr,
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
};

use crate::join::{JoinedToUIMessage, UIToJoinedMessage};

#[derive(Debug, Default)]
struct JoinState {
    message_sender: Rc<Mutex<Option<Sender<UIToJoinedMessage>>>>,
    message_receiver: Mutex<Option<Receiver<JoinedToUIMessage>>>,
}

pub fn build_page() -> impl IsA<Widget> {
    let title = Label::builder()
        .label("Join")
        .css_classes(["title-1"])
        .build();

    let address_label = Label::builder()
        .label("Address")
        .halign(Align::Start)
        .build();
    let address_buffer = EntryBuffer::new(None::<String>);
    let address_input = Entry::builder()
        .placeholder_text("111.222.333.444")
        .input_purpose(libadwaita::gtk::InputPurpose::Digits)
        .max_length(15)
        .buffer(&address_buffer)
        .build();
    let address_box = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .spacing(4)
        .halign(Align::Center)
        .width_request(200)
        .build();
    address_box.append(&address_label);
    address_box.append(&address_input);

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

    let join_button = Button::builder()
        .label("Join")
        .css_classes(["suggested-action"])
        .width_request(200)
        .halign(Align::Center)
        .build();

    let join_page = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .valign(Align::Center)
        .spacing(16)
        .build();
    join_page.append(&title);
    join_page.append(&address_box);
    join_page.append(&port_box);
    join_page.append(&join_button);

    let title = Label::builder()
        .label("Joined")
        .css_classes(["title-1"])
        .build();

    let leave_button = Button::builder()
        .label("Leave")
        .css_classes(["destructive-action"])
        .width_request(200)
        .halign(Align::Center)
        .build();

    let joined_page = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .valign(Align::Center)
        .spacing(16)
        .build();
    joined_page.append(&title);
    joined_page.append(&leave_button);

    let stack = Stack::new();
    stack.add_named(&join_page, Some("join-page"));
    stack.add_named(&joined_page, Some("joined-page"));

    let state = JoinState::default();

    let stack_clone = stack.clone();
    let sender_clone = state.message_sender.clone();
    join_button.connect_clicked(move |_| {
        if port_buffer.text().len() < 4
            || std::net::IpAddr::from_str(&address_buffer.text()).is_err()
        {
            return;
        }
        let (sender, receiver) = start_joining(
            address_buffer.text().to_string(),
            port_buffer.text().to_string(),
        );
        *sender_clone.lock().unwrap() = Some(sender);
        *state.message_receiver.lock().unwrap() = Some(receiver);
        stack_clone.set_visible_child(&joined_page);
    });

    let stack_clone = stack.clone();
    let sender_clone = state.message_sender.clone();
    leave_button.connect_clicked(move |_| {
        sender_clone
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .send(UIToJoinedMessage::Leave)
            .unwrap();
        stack_clone.set_visible_child(&join_page);
    });

    stack
}

fn start_joining(
    address_string: String,
    port_string: String,
) -> (Sender<UIToJoinedMessage>, Receiver<JoinedToUIMessage>) {
    let address = std::net::IpAddr::from_str(&address_string).unwrap();
    let port: u16 = port_string.parse().unwrap();
    println!("Joining {} at port {}", address, port);

    let (sender0, receiver0) = mpsc::channel::<JoinedToUIMessage>();
    let (sender1, receiver1) = mpsc::channel::<UIToJoinedMessage>();

    std::thread::spawn(move || crate::join::join(address, port, sender0, receiver1));
    (sender1, receiver0)
}
