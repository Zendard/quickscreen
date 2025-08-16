use scap::capturer::Capturer;

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
        output_type: scap::frame::FrameType::BGRAFrame,
        output_resolution: scap::capturer::Resolution::_720p,
        ..Default::default()
    };
    Ok(Capturer::build(options)?)
}
