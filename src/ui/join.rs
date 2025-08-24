use libadwaita::{
    AlertDialog,
    gio::Cancellable,
    glib::object::{IsA, ObjectExt},
    gtk::{
        Align, Button, Entry, EntryBuffer, Label, Stack, Widget,
        prelude::{BoxExt, ButtonExt, EditableExt, EditableExtManual, EntryBufferExtManual},
    },
    prelude::{AlertDialogExt, AlertDialogExtManual},
};
use std::{
    rc::Rc,
    str::FromStr,
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

use crate::join::{JoinedToUIMessage, UIToJoinedMessage};

#[derive(Debug, Default, Clone)]
struct JoinState {
    message_sender: Rc<Mutex<Option<Sender<UIToJoinedMessage>>>>,
    message_receiver: Rc<Mutex<Option<Receiver<JoinedToUIMessage>>>>,
    join_request_response_dialog: AlertDialog,
    parent_widget: Stack,
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
        .label("Requesting to join...")
        .css_classes(["title-1"])
        .build();

    let cancel_button = Button::builder()
        .label("Cancel")
        .css_classes(["destructive-action"])
        .width_request(200)
        .halign(Align::Center)
        .build();

    let requesting_page = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .valign(Align::Center)
        .spacing(16)
        .build();
    requesting_page.append(&title);
    requesting_page.append(&cancel_button);

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

    let join_request_response_dialog = AlertDialog::builder()
        .title("Join request response")
        .heading("Unknown response")
        .body("You were unknown")
        .close_response("ok")
        .build();
    join_request_response_dialog.add_response("ok", "Ok");

    let joined_page = libadwaita::gtk::Box::builder()
        .orientation(libadwaita::gtk::Orientation::Vertical)
        .valign(Align::Center)
        .spacing(16)
        .build();
    joined_page.append(&title);
    joined_page.append(&leave_button);

    let stack = Stack::new();
    stack.add_titled(&join_page, Some("join-page"), "Join");
    stack.add_titled(
        &requesting_page,
        Some("requesting-page"),
        "Requesting to join...",
    );
    stack.add_titled(&joined_page, Some("joined-page"), "Joined");

    let state = JoinState {
        join_request_response_dialog,
        parent_widget: stack.clone(),
        ..Default::default()
    };

    let stack_clone = stack.clone();
    let state_clone = state.clone();
    join_button.connect_clicked(move |_| {
        if port_buffer.text().len() < 4
            || std::net::IpAddr::from_str(&address_buffer.text()).is_err()
        {
            return;
        }
        let (sender, receiver) = start_joining(
            address_buffer.text().to_string(),
            port_buffer.text().to_string(),
            &state_clone,
        );
        *state_clone.message_sender.lock().unwrap() = Some(sender);
        *state_clone.message_receiver.lock().unwrap() = Some(receiver);
        stack_clone.set_visible_child(&requesting_page);
    });

    let stack_clone = stack.clone();
    let sender_clone = state.message_sender.clone();
    cancel_button.connect_clicked(move |_| {
        sender_clone
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .send(UIToJoinedMessage::Leave)
            .unwrap();
        stack_clone.set_visible_child_name("join-page");
    });

    let sender_clone = state.message_sender.clone();
    let stack_clone = stack.clone();
    leave_button.connect_clicked(move |_| {
        sender_clone
            .lock()
            .unwrap()
            .clone()
            .unwrap()
            .send(UIToJoinedMessage::Leave)
            .unwrap();
        stack_clone.set_visible_child_name("join-page");
    });

    stack
}

fn start_joining(
    address_string: String,
    port_string: String,
    state: &JoinState,
) -> (Sender<UIToJoinedMessage>, Receiver<JoinedToUIMessage>) {
    let address = std::net::IpAddr::from_str(&address_string).unwrap();
    let port: u16 = port_string.parse().unwrap();
    println!("Joining {} at port {}", address, port);

    let (sender0, receiver0) = mpsc::channel::<JoinedToUIMessage>();
    let (sender1, receiver1) = mpsc::channel::<UIToJoinedMessage>();

    std::thread::spawn(move || crate::join::join(address, port, sender0, receiver1));

    let mut state_clone = state.clone();
    libadwaita::glib::timeout_add_local(Duration::from_millis(100), move || {
        listen_for_message(&mut state_clone);
        libadwaita::glib::ControlFlow::Continue
    });
    (sender1, receiver0)
}

fn listen_for_message(state: &mut JoinState) {
    let receiver_clone = state.message_receiver.clone();
    let mut receiver = receiver_clone.lock().unwrap();
    if receiver.is_none() {
        return;
    }
    let receiver = receiver.as_mut().unwrap();
    if let Ok(message) = receiver.try_recv() {
        match message {
            JoinedToUIMessage::JoinRequestResponse(accepted) => {
                handle_join_request_response(accepted, state)
            }
        }
    }
}

fn handle_join_request_response(accepted: bool, state: &JoinState) {
    state.join_request_response_dialog.clone().choose(
        &state.parent_widget,
        None::<&Cancellable>,
        |_| {},
    );
    if accepted {
        state
            .join_request_response_dialog
            .set_heading(Some("Accepted"));
        state
            .join_request_response_dialog
            .set_body("You were accepted");
        state.parent_widget.set_visible_child_name("joined-page");
    } else {
        state
            .join_request_response_dialog
            .set_heading(Some("Refused"));
        state
            .join_request_response_dialog
            .set_body("You were refused");
        state
            .message_sender
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(UIToJoinedMessage::Leave)
            .unwrap();
        state.parent_widget.set_visible_child_name("join-page");
    }
}
