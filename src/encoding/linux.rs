use ashpd::desktop::{
    PersistMode,
    screencast::{CursorMode, Screencast, SourceType, Stream as ScreencastStream},
};
use pipewire::{main_loop::MainLoop, properties::properties};
use pollster::FutureExt;
use std::os::fd::OwnedFd;

struct FrameData {
    format: pipewire::spa::param::video::VideoInfoRaw,
}

async fn open_portal() -> ashpd::Result<(ScreencastStream, OwnedFd)> {
    let proxy = Screencast::new().block_on()?;
    let session = proxy.create_session().block_on()?;
    proxy
        .select_sources(
            &session,
            CursorMode::Hidden,
            SourceType::Monitor.into(),
            false,
            None,
            PersistMode::DoNot,
        )
        .block_on()?;

    let response = proxy.start(&session, None).block_on()?.response()?;
    let stream = response
        .streams()
        .first()
        .expect("no stream found / selected")
        .to_owned();

    let fd = proxy.open_pipe_wire_remote(&session).block_on()?;

    Ok((stream, fd))
}

async fn start_pipewire_stream(
    node_id: u32,
    fd: OwnedFd,
    app_source: gstreamer_app::AppSrc,
) -> Result<(MainLoop, pipewire::stream::Stream), pipewire::Error> {
    println!("Starting stream");
    pipewire::init();

    let mainloop = pipewire::main_loop::MainLoop::new(None)?;
    let context = pipewire::context::Context::new(&mainloop)?;
    let core = context.connect_fd(fd, None)?;

    let data = FrameData {
        format: Default::default(),
    };

    let stream = pipewire::stream::Stream::new(
        &core,
        "video-test",
        properties! {
            *pipewire::keys::MEDIA_TYPE => "Video",
            *pipewire::keys::MEDIA_CATEGORY => "Capture",
            *pipewire::keys::MEDIA_ROLE => "Screen",
        },
    )?;

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .state_changed(|_, _, old, new| {
            println!("State changed: {:?} -> {:?}", old, new);
        })
        .param_changed(|_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != pipewire::spa::param::ParamType::Format.as_raw() {
                return;
            }

            let (media_type, media_subtype) =
                match pipewire::spa::param::format_utils::parse_format(param) {
                    Ok(v) => v,
                    Err(_) => return,
                };

            if media_type != pipewire::spa::param::format::MediaType::Video
                || media_subtype != pipewire::spa::param::format::MediaSubtype::Raw
            {
                return;
            }

            user_data
                .format
                .parse(param)
                .expect("Failed to parse param changed to VideoInfoRaw");

            println!("got video format:");
            println!(
                "\tformat: {} ({:?})",
                user_data.format.format().as_raw(),
                user_data.format.format()
            );
            println!(
                "\tsize: {}x{}",
                user_data.format.size().width,
                user_data.format.size().height
            );
            println!(
                "\tframerate: {}/{}",
                user_data.format.framerate().num,
                user_data.format.framerate().denom
            );

            // prepare to render video of this size
        })
        .process(move |stream, _| match stream.dequeue_buffer() {
            None => println!("Out of buffers"),
            Some(mut buffer) => {
                // println!("Pushing buffer to app_source");
                let slice = buffer.datas_mut().first_mut().unwrap().data().unwrap();
                let mut buffer = gstreamer::Buffer::with_size(slice.len()).unwrap();
                {
                    let buffer = buffer.get_mut().unwrap();
                    buffer.copy_from_slice(0, slice).unwrap();
                }
                app_source.push_buffer(buffer).unwrap();
            }
        })
        .register()?;

    println!("Created stream {:#?}", stream);

    let obj = pipewire::spa::pod::object!(
        pipewire::spa::utils::SpaTypes::ObjectParamFormat,
        pipewire::spa::param::ParamType::EnumFormat,
        pipewire::spa::pod::property!(
            pipewire::spa::param::format::FormatProperties::MediaType,
            Id,
            pipewire::spa::param::format::MediaType::Video
        ),
        pipewire::spa::pod::property!(
            pipewire::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pipewire::spa::param::format::MediaSubtype::Raw
        ),
        pipewire::spa::pod::property!(
            pipewire::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            pipewire::spa::param::video::VideoFormat::RGB,
            pipewire::spa::param::video::VideoFormat::RGB,
            pipewire::spa::param::video::VideoFormat::RGBA,
            pipewire::spa::param::video::VideoFormat::RGBx,
            pipewire::spa::param::video::VideoFormat::BGRx,
            pipewire::spa::param::video::VideoFormat::YUY2,
            pipewire::spa::param::video::VideoFormat::I420,
        ),
        pipewire::spa::pod::property!(
            pipewire::spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            pipewire::spa::utils::Rectangle {
                width: 320,
                height: 240
            },
            pipewire::spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            pipewire::spa::utils::Rectangle {
                width: 4096,
                height: 4096
            }
        ),
        pipewire::spa::pod::property!(
            pipewire::spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            pipewire::spa::utils::Fraction { num: 25, denom: 1 },
            pipewire::spa::utils::Fraction { num: 0, denom: 1 },
            pipewire::spa::utils::Fraction {
                num: 1000,
                denom: 1
            }
        ),
    );
    let values: Vec<u8> = pipewire::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pipewire::spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [pipewire::spa::pod::Pod::from_bytes(&values).unwrap()];

    stream.connect(
        pipewire::spa::utils::Direction::Input,
        Some(node_id),
        pipewire::stream::StreamFlags::AUTOCONNECT | pipewire::stream::StreamFlags::MAP_BUFFERS,
        &mut params,
    )?;

    println!("Connected stream");
    mainloop.run();

    Ok((mainloop, stream))
}

pub fn new_source()
-> Result<(gstreamer_app::AppSrc, ScreencastStream, OwnedFd), Box<dyn std::error::Error>> {
    let (stream, fd) = open_portal().block_on()?;

    let resolution = stream.size().ok_or("Stream has no size")?;
    let video_info = gstreamer_video::VideoInfo::builder(
        gstreamer_video::VideoFormat::Bgrx,
        resolution.0 as u32,
        resolution.1 as u32,
    )
    .fps(gstreamer::Fraction::new(2, 1))
    .build()
    .expect("Failed to create video info");

    let source = gstreamer_app::AppSrc::builder()
        .caps(&video_info.to_caps().unwrap())
        .format(gstreamer::Format::Time)
        .build();

    Ok((source, stream, fd))
}

pub fn start(source: gstreamer_app::AppSrc, stream: ScreencastStream, fd: OwnedFd) {
    let pipewire_node_id = stream.pipe_wire_node_id();

    std::thread::spawn(move || {
        start_pipewire_stream(pipewire_node_id, fd, source)
            .block_on()
            .unwrap();
    });
}
