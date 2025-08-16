use std::sync::mpsc::{Receiver, Sender};

mod capture;

pub enum HostingToUIMessage {}
pub enum UIToHostingMessage {
    Stop,
}

pub fn host(
    port: u16,
    message_sender: Sender<HostingToUIMessage>,
    message_receiver: Receiver<UIToHostingMessage>,
) {
    let mut capturer = capture::new().unwrap();
    capturer.start_capture();
    loop {
        if let Ok(message) = message_receiver.try_recv() {
            match message {
                UIToHostingMessage::Stop => break,
            }
        }
        let frame = capturer.get_next_frame().unwrap();
        dbg!(frame);
    }
    capturer.start_capture();
    println!("Stopped hosting");
}
