use crate::host::{HostingToUIMessage, UIToHostingMessage, network::ClientID};
use libadwaita::{
    AlertDialog,
    gio::Cancellable,
    glib::{
        GString,
        object::{IsA, ObjectExt},
    },
    gtk::{
        Align, Button, Entry, EntryBuffer, Label, Stack, Widget,
        prelude::{BoxExt, ButtonExt, EditableExt, EditableExtManual, EntryBufferExtManual},
    },
    prelude::{AlertDialogExt, AlertDialogExtManual},
};
use std::{
    rc::Rc,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

#[derive(Debug, Default, Clone)]
struct HostState {
    message_sender: Rc<Mutex<Option<Sender<UIToHostingMessage>>>>,
    message_receiver: Arc<Mutex<Option<Receiver<HostingToUIMessage>>>>,
    join_request_dialog: AlertDialog,
    parent_widget: Stack,
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

    let join_request_dialog = AlertDialog::builder()
        .title("Join request")
        .heading("Join request")
        .body("Client unknown wants to join")
        .close_response("refuse")
        .default_response("accept")
        .build();
    join_request_dialog.add_responses(&[("refuse", "Refuse"), ("accept", "Accept")]);
    join_request_dialog
        .set_response_appearance("refuse", libadwaita::ResponseAppearance::Destructive);
    join_request_dialog
        .set_response_appearance("accept", libadwaita::ResponseAppearance::Suggested);

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

    let mut state = HostState {
        join_request_dialog: join_request_dialog.clone(),
        ..Default::default()
    };

    let stack_clone = stack.clone();
    let state_clone = state.clone();
    host_button.connect_clicked(move |_| {
        if port_buffer.text().len() < 4 {
            return;
        }
        let (sender, receiver) = start_hosting(port_buffer.text().to_string(), &state_clone);
        *state_clone.message_sender.lock().unwrap() = Some(sender);
        *state_clone.message_receiver.lock().unwrap() = Some(receiver);
        stack_clone.set_visible_child(&hosting_page);
    });

    let stack_clone = stack.clone();
    let sender_clone = state.message_sender.clone();
    stop_button.connect_clicked(move |_| {
        sender_clone
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .send(UIToHostingMessage::Stop)
            .unwrap();
        stack_clone.set_visible_child(&host_page);
    });
    state.parent_widget = stack.clone();

    stack
}

fn start_hosting(
    port_string: String,
    state: &HostState,
) -> (Sender<UIToHostingMessage>, Receiver<HostingToUIMessage>) {
    let port: u16 = port_string.parse().unwrap();
    println!("Hosting on port {}", port_string);

    let (sender0, receiver0) = mpsc::channel::<HostingToUIMessage>();
    let (sender1, receiver1) = mpsc::channel::<UIToHostingMessage>();

    std::thread::spawn(move || crate::host::host(port, sender0, receiver1));
    let mut state_clone = state.clone();
    libadwaita::glib::timeout_add_local(Duration::from_millis(100), move || {
        listen_for_message(&mut state_clone);
        libadwaita::glib::ControlFlow::Continue
    });
    (sender1, receiver0)
}

fn listen_for_message(state: &mut HostState) {
    let receiver_clone = state.message_receiver.clone();
    let mut receiver = receiver_clone.lock().unwrap();
    if receiver.is_none() {
        return;
    }
    let receiver = receiver.as_mut().unwrap();
    if let Ok(message) = receiver.try_recv() {
        match message {
            HostingToUIMessage::JoinRequest(client_id) => handle_join_request(client_id, state),
        }
    }
}

fn handle_join_request(client_id: ClientID, state: &HostState) {
    println!("Showing join request dialog");
    let message_sender = state
        .message_sender
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    state.join_request_dialog.clone().choose(
        &state.parent_widget,
        None::<&Cancellable>,
        move |choice| handle_join_request_respone(choice, message_sender, client_id),
    );
}

fn handle_join_request_respone(
    choice: GString,
    message_sender: Sender<UIToHostingMessage>,
    client_id: ClientID,
) {
    dbg!(&message_sender);
    if choice == "accept" {
        println!("Sending accepted");
        message_sender
            .send(UIToHostingMessage::JoinRequestResponse(client_id, true))
            .unwrap()
    } else {
        println!("Sending refused");
        message_sender
            .send(UIToHostingMessage::JoinRequestResponse(client_id, false))
            .unwrap()
    }
}
