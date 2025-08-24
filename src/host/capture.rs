use scap::{
    capturer::Capturer,
    frame::{BGRxFrame, Frame},
};

#[derive(Debug)]
pub struct NetworkFrame {
    pub data: Vec<(u8, u8, u8)>,
}

pub fn new() -> Result<Capturer, Box<dyn std::error::Error>> {
    if !scap::is_supported() {
        return Err("Platform not supported".into());
    }

    if !scap::has_permission() {
        println!("Requesting permission to capture screen...");
        if !scap::request_permission() {
            return Err("Permission denied".into());
        }
    }

    let options = scap::capturer::Options {
        fps: 30,
        target: None,
        show_cursor: true,
        show_highlight: true,
        excluded_targets: None,
        output_type: scap::frame::FrameType::RGB,
        output_resolution: scap::capturer::Resolution::_720p,
        ..Default::default()
    };
    Ok(Capturer::build(options)?)
}

impl From<Frame> for NetworkFrame {
    fn from(value: Frame) -> Self {
        match value {
            Frame::BGRx(frame) => frame.into(),
            _ => todo!("Support more frame types"),
        }
    }
}

impl From<BGRxFrame> for NetworkFrame {
    fn from(value: BGRxFrame) -> Self {
        let mut output = Vec::with_capacity(value.data.len() / 3);
        for i in 0..value.data.len() / 3 {
            output.push((value.data[i], value.data[i + 1], value.data[i + 2]))
        }
        Self { data: output }
    }
}
