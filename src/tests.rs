use libadwaita::gtk::cairo::Surface;

use crate::host::capture::{self, NetworkFrame, RESOLUTION};

#[test]
fn convert_frame_to_network_frame() {
    let mut capturer = capture::new().unwrap();
    capturer.start_capture();
    let frame = capturer.get_next_frame().unwrap();
    capturer.stop_capture();

    let network_frame: NetworkFrame = frame.into();
    assert_eq!(network_frame.data.len(), RESOLUTION.0 * RESOLUTION.1 * 3)
}

#[test]
fn convert_network_frame_to_surface() {
    let mut capturer = capture::new().unwrap();
    capturer.start_capture();
    let frame = capturer.get_next_frame().unwrap();
    capturer.stop_capture();

    let network_frame: NetworkFrame = frame.into();
    let _: Surface = network_frame.into();
}
