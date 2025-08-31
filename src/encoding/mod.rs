use gstreamer::{
    Pipeline,
    glib::object::Cast,
    prelude::{ElementExt, GstBinExtManual},
};
use std::sync::Mutex;

#[cfg(target_os = "linux")]
pub mod linux;
pub mod network;

pub const RESOLUTION: (usize, usize) = (1920, 1080);
const BITRATE: u32 = 256;

#[derive(Debug)]
pub struct NetworkFrame {
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct Encoder {
    pub pipeline: Pipeline,
    pub sink: gstreamer::Element,
    pub frame_index: Mutex<u64>,
}

impl Encoder {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        gstreamer::init()?;
        let pipeline = gstreamer::Pipeline::new();

        #[cfg(target_os = "linux")]
        let (source, stream, fd) = linux::new_source()?;

        let video_convert = gstreamer::ElementFactory::make("videoconvert").build()?;

        let encoder = gstreamer::ElementFactory::make("x265enc")
            .property("bitrate", BITRATE)
            .property("key-int-max", 1)
            .build()?;

        let parser = gstreamer::ElementFactory::make("h265parse").build()?;

        let queue = gstreamer::ElementFactory::make("queue").build()?;

        // let sink = gstreamer::ElementFactory::make("udpsink")
        //     .property("host", "0.0.0.0")
        //     .property("port", 1234)
        //     .build()?;

        let sink = gstreamer::ElementFactory::make("autovideosink").build()?;

        pipeline.add_many(&[
            source.upcast_ref(),
            &video_convert,
            &encoder,
            &parser,
            &queue,
            &sink,
        ])?;

        gstreamer::Element::link_many(&[
            source.upcast_ref(),
            &video_convert,
            // &encoder,
            // &parser,
            // &queue,
            &sink,
        ])?;

        pipeline.set_state(gstreamer::State::Playing).unwrap();

        #[cfg(target_os = "linux")]
        linux::start(source, stream, fd);

        Ok(Self {
            pipeline,
            sink,
            frame_index: Mutex::new(0),
        })
    }
}
