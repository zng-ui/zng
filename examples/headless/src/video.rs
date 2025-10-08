use std::io::Write as _;
use window::FrameCaptureMode;
use zng::prelude::*;

/// This example uses a headless window to render frames for FFmpeg.
pub fn run() {
    println!("-=Headless Example (video)=-\n");

    // open headless with renderer flag, this causes the view-process to start.
    let mut app = APP.defaults().run_headless(true);
    // saving frame can be slow, so we will manually control the app time to not miss any frame.
    APP.start_manual_time();

    const FPS: f32 = 60.0;
    zng::var::VARS.frame_duration().set((1.0 / FPS).secs());

    app.run_window(async {
        // will save frames as "{temp}/{frame}.png"
        let temp = zng::env::cache("headless_example_video");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();

        let frame = var(0u32);
        let recorded = var(false);
        print_status("recording", &recorded);

        Window! {
            // the window content is the "video".
            child = video(recorded.clone());
            auto_size = true;

            // use the CPU only backend if available, by default the
            // same GPU used for headed windows is used.
            render_mode = window::RenderMode::Software;

            // capture all frames.
            frame_capture_mode = FrameCaptureMode::All;

            // this event will fire every time a frame is rendered.
            on_frame_image_ready = async_hn!(temp, frame, |args| {
                let img = args.frame_image.unwrap();

                let frame_i = frame.get();
                frame.set(frame_i + 1);

                img.save(temp.join(format!("{frame_i:05}.png"))).await.unwrap();

                // advance time at a perfect framerate.
                APP.advance_manual_time((1.0 / FPS).secs());
                // ensure a frame image is actually generated (for video).
                //
                // also, retained rendering only renders when needed, so without this
                // line the app never even updates, and the initial delay timer waits forever.
                WIDGET.render_update();
            });

            on_load = async_hn!(recorded, temp, |_| {
                recorded.wait_match(|&f| f).await;

                let encoded = var(false);
                print_status("encoding ./screencast.mp4 ", &encoded);

                task::spawn_wait(clmv!(encoded, || {
                    // https://www.ffmpeg.org/download.html
                    let ffmpeg = std::process::Command::new("ffmpeg")
                        .arg("-framerate")
                        .arg(FPS.to_string())
                        .arg("-y")
                        .arg("-i")
                        .arg(temp.join("%05d.png"))
                        .arg("-c:v")
                        .arg("libx264")
                        .arg("-pix_fmt")
                        .arg("yuv420p")
                        .arg("screencast.mp4")
                        .arg("-hide_banner")
                        .arg("-loglevel")
                        .arg("error")
                        .status();
                    let _ = std::fs::remove_dir_all(temp);
                    encoded.set(true);

                    match ffmpeg {
                        Ok(ffmpeg) => assert!(ffmpeg.success()),
                        Err(e) => panic!("cannot run 'ffmpeg', {e}"),
                    }
                }));
                encoded.wait_match(|&f| f).await;
                println!("\rencoding ./screencast.mp4 ... done");

                APP.exit();
            });
        }
    });
    while !matches!(app.update(true), zng::app::AppControlFlow::Exit) {}
}

fn video(finished: Var<bool>) -> UiNode {
    let bkg_rotate = var(0.turn());
    let txt_fade = var(0.fct());
    let txt_size = var(32.dip());
    let fade_out = var(0.fct());
    Container! {
        layout::size = (800, 600);

        widget::on_init = async_hn!(txt_fade, txt_size, bkg_rotate, fade_out, finished, |_| {
            task::deadline(300.ms()).await;
            txt_fade.ease(1.fct(), 800.ms(), easing::linear).perm();
            txt_size.ease(72, 800.ms(), easing::linear).perm();

            task::deadline(100.ms()).await;
            bkg_rotate.ease(5.turn(), 10.secs(), easing::circ).perm();

            task::deadline(8.secs()).await;
            txt_size.ease(120, 2.secs(), easing::linear).perm();
            txt_fade.ease(0.fct(), 2.secs(), easing::linear).perm();

            task::deadline(1.secs()).await;
            fade_out.ease(1.fct(), 1.secs(), easing::linear).perm();

            bkg_rotate.wait_animation().await;
            finished.set(true);
        });

        widget::background = Stack! {
            children = {
                let gradient = clmv!(bkg_rotate, |angle: i32, mut color: color::Rgba| {
                    color.alpha = 0.3;
                    let stops = color::gradient::GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                    let angle = angle.deg();
                    color::gradient::linear_gradient(bkg_rotate.map(move |r| (angle + layout::AngleDegree::from(*r)).into()), stops)
                });

                ui_vec![
                    color::flood(colors::WHITE),
                    gradient(0, colors::RED),
                    gradient(20, colors::RED),
                    gradient(40, colors::RED),
                    gradient(120, colors::GREEN),
                    gradient(140, colors::GREEN),
                    gradient(160, colors::GREEN),
                    gradient(240, colors::BLUE),
                    gradient(260, colors::BLUE),
                    gradient(280, colors::BLUE),
                ]
            };
        };
        widget::foreground_color = fade_out.map(|&o| colors::BLACK.with_alpha(o));

        child = Text! {
            layout::align = Align::CENTER;
            txt = "Hello World!";
            font_family = ["Consolas", "monospace"];
            font_color = colors::WHITE;
            font_size = txt_size;
            color::filter::opacity = txt_fade;
        };
    }
}

fn print_status(task: &'static str, done: &Var<bool>) {
    task::spawn(async_clmv!(done, {
        let mut dots = 0;
        while !done.get() {
            dots += 1;
            if dots > 3 {
                dots = 0;
            }
            print!("\r                                         ");
            print!("\r{task}{}", String::from_utf8(vec![b'.'; dots]).unwrap());
            std::io::stdout().lock().flush().ok();
            task::deadline(500.ms()).await;
        }
    }));
}
