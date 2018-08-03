#[macro_use]
extern crate log;
extern crate dotenv;
extern crate egg_mode;
extern crate env_logger;
extern crate futures;
extern crate hyper;
extern crate hyper_rustls;
extern crate image;
extern crate postgres;
extern crate rusttype;
extern crate tokio_core;

use dotenv::dotenv;
use egg_mode::{
    tweet::{user_timeline, Timeline, Tweet},
    KeyPair, Token,
};
use futures::future::{Executor, Future};
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
use image::{DynamicImage, GenericImage, Pixel, Rgba, RgbaImage};
use rusttype::{point, Font, Scale};
use std::env;
use std::time::Duration;
use tokio_core::reactor::{Core, Timeout};

mod state;
use state::State;

const FONT: &[u8] = include_bytes!("../impact.ttf");
const FONT_SIZE: f32 = 128.0;
const BORDER_SIZE: u32 = 6;
const TEXT_MARGIN: f32 = 20.0;
const OUTER_MARGIN: u32 = 10;
const TIMELINE_PAGE_SIZE: i32 = 10;
const INTERVAL: Duration = Duration::from_secs(1800);

const INDEX_HTML: &[u8] = include_bytes!("../index.html");

fn _test_main() {
    let font = Font::from_bytes(FONT).unwrap();

    let mut im = image::open("cat.jpg").expect("Unable to load image.");
    im = draw_on_image(&im, &font, "KHAJIIT HAS WARES", "IF YOU HAVE COIN");
    im.save("cat2.jpg").expect("Unable to save image.");
}

fn main() {
    dotenv().ok();
    env_logger::init();

    loop {
        match main_process() {
            Err(err) => error!("{:?}", err),
            _ => {
                trace!("Process exited without incident.");
                break;
            }
        }
        warn!("Restarting process.");
    }
}

#[allow(unknown_lints)]
#[allow(needless_pass_by_value)]
fn web_responder(_req: Request<Body>) -> Response<Body> {
    Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(
            ::std::str::from_utf8(INDEX_HTML).expect("index.html has invalid UTF-8!"),
        )).unwrap()
}

fn run_server(core: &mut Core) -> Result<(), String> {
    let port = env::var("PORT")
        .expect("No PORT environment variable set.")
        .parse()
        .expect("Unable to parse value of PORT environment variable.");
    let addr = ([127, 0, 0, 1], port).into();
    let responder = || service_fn_ok(web_responder);
    let server = Server::bind(&addr)
        .serve(responder)
        .map_err(|e| error!("Web server error: {}", e));
    core.execute(server)
        .map_err(|error| format!("Failed to start web server: {:?}", error))
}

fn main_process() -> Result<(), String> {
    let mut state = State::new()?;

    let font = Font::from_bytes(FONT).unwrap();

    let consumer = KeyPair::new(
        env::var("CONSUMER_KEY").expect("No CONSUMER_KEY environment variable set."),
        env::var("CONSUMER_SECRET").expect("No CONSUMER_SECRET environment variable set."),
    );
    let access = KeyPair::new(
        env::var("ACCESS_TOKEN").expect("No ACCESS_TOKEN environment variable set."),
        env::var("ACCESS_SECRET").expect("No ACCESS_SECRET environment variable set."),
    );
    let token = Token::Access { consumer, access };

    let mut core = Core::new().expect("Could not set up Tokio reactor.");
    run_server(&mut core)?;
    let handle = core.handle();

    let user = core
        .run(egg_mode::verify_tokens(&token, &handle))
        .expect("ABORTING: Twitter token no longer valid!");
    info!("Logged in as: {:?}", user.screen_name);
    loop {
        iterate(&mut core, &mut state, &token, &font)?;
        let timeout = match Timeout::new(INTERVAL, &handle) {
            Ok(timeout) => timeout,
            Err(error) => return Err(format!("Error creating timeout: {:?}", error)),
        };
        match core.run(timeout) {
            Ok(_) => (),
            Err(error) => return Err(format!("Timeout failed! {:?}", error)),
        }
    }
}

fn iterate(core: &mut Core, state: &mut State, token: &Token, font: &Font) -> Result<(), String> {
    let handle = core.handle();
    let timeline = user_timeline("Bodegacats_", false, false, token, &handle)
        .with_page_size(TIMELINE_PAGE_SIZE);
    let (_timeline, images) = check_timeline(core, state, timeline)?;
    if env::var("DRY_RUN").is_err() {
        for image_url in images {
            let img = process_image(core, &image_url, |i| {
                draw_on_image(&i, font, "KHAJIIT HAS WARES", "IF YOU HAVE COIN")
            })?;
            send_tweet(core, token, img)?;
        }
    }

    Ok(())
}

fn check_timeline<'a>(
    core: &mut Core,
    state: &mut State,
    timeline: Timeline<'a>,
) -> Result<(Timeline<'a>, Vec<String>), String> {
    trace!("Checking timeline.");
    match core.run(timeline.start()) {
        Ok((new_timeline, response)) => {
            let mut out = Vec::new();
            for tweet in response.response.iter().rev() {
                out.extend(process_tweet(tweet, state)?);
            }
            Ok((new_timeline, out))
        }
        Err(error) => Err(format!("Unable to load target timeline: {:?}", error)),
    }
}

fn process_tweet(tweet: &Tweet, state: &mut State) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    if state.insert(tweet.id)? {
        info!("Processing new tweet: {:?}", tweet.text);
        if let Some(ref entities) = tweet.entities.media {
            for entity in entities {
                out.push(entity.media_url_https.to_owned());
            }
        }
    }
    Ok(out)
}

fn load_url(core: &mut Core, url: &str) -> Result<Vec<u8>, String> {
    use hyper::{rt::Stream, Body, Client, Uri};

    trace!("Loading image from URL: {}", url);

    let uri = match url.parse::<Uri>() {
        Ok(uri) => uri,
        Err(error) => return Err(format!("Failed to parse URL {:?}: {:?}", url, error)),
    };

    let https = hyper_rustls::HttpsConnector::new(4);
    let client: Client<_, Body> = Client::builder().build(https);
    match core.run(client.get(uri)) {
        Ok(res) => {
            if !res.status().is_success() {
                return Err(format!("URL {:?} gave status code {:?}", url, res.status()));
            }
            let body = res.into_body();
            match body.concat2().wait() {
                Ok(data) => Ok(data.to_vec()),
                Err(error) => Err(format!("Failed to read body of URL {:?}: {:?}", url, error)),
            }
        }
        Err(error) => Err(format!("Failed to read URL {:?}: {:?}", url, error)),
    }
}

fn process_image<F>(core: &mut Core, url: &str, f: F) -> Result<Vec<u8>, String>
where
    F: Fn(DynamicImage) -> DynamicImage,
{
    let image_data = load_url(core, url)?;
    trace!("Processing image...");
    let mut im = match image::load_from_memory(&image_data) {
        Ok(image) => image,
        Err(error) => {
            return Err(format!(
                "Unable to decode image loaded from {:?}: {:?}",
                url, error
            ))
        }
    };

    im = f(im);

    let mut out = Vec::new();
    if let Err(error) = im.write_to(&mut out, image::ImageOutputFormat::JPEG(95)) {
        return Err(format!("PANIC: Unable to encode image: {:?}", error));
    }

    Ok(out)
}

fn send_tweet(core: &mut Core, token: &Token, img: Vec<u8>) -> Result<(), String> {
    use egg_mode::media::{media_types::image_jpg, UploadBuilder};
    use egg_mode::tweet::DraftTweet;

    info!("Tweeting image of size {}", img.len());

    let handle = core.handle();
    match core.run(
        UploadBuilder::new(img, image_jpg())
            .alt_text("Khajiit has wares, if you have coin.")
            .call(token, &handle),
    ) {
        Err(error) => Err(format!("Failed to upload image: {:?}", error)),
        Ok(media_handle) => match core.run(
            DraftTweet::new("")
                .media_ids(&[media_handle.id])
                .send(token, &handle),
        ) {
            Err(error) => Err(format!("Failed to send tweet: {:?}", error)),
            Ok(_) => Ok(()),
        },
    }
}

fn render_text(font: &Font, text: &str) -> RgbaImage {
    let scale = Scale::uniform(FONT_SIZE);
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font
        .layout(
            text,
            scale,
            point(TEXT_MARGIN, TEXT_MARGIN + v_metrics.ascent),
        ).collect();

    let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
    let glyphs_width = {
        let min_x = glyphs
            .first()
            .map(|g| g.pixel_bounding_box().unwrap().min.x)
            .unwrap();
        let max_x = glyphs
            .last()
            .map(|g| g.pixel_bounding_box().unwrap().max.x)
            .unwrap();
        (max_x - min_x) as u32
    };

    let mut im = DynamicImage::new_rgba8(glyphs_width + 40, glyphs_height + 40).to_rgba();
    // FIXME: This is probably unnecessary?
    for pixel in im.pixels_mut() {
        *pixel = Rgba { data: [0, 0, 0, 0] }
    }

    for glyph in &glyphs {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            glyph.draw(|px, py, v| {
                for xoff in 0..=(BORDER_SIZE * 2) {
                    for yoff in 0..=(BORDER_SIZE * 2) {
                        let x = px + bounds.min.x as u32 + xoff - BORDER_SIZE;
                        let y = py + bounds.min.y as u32 + yoff - BORDER_SIZE;
                        let old_p = im.get_pixel_mut(x, y);
                        let new_p = Rgba {
                            data: [0, 0, 0, (v * 255.0) as u8],
                        };
                        old_p.blend(&new_p);
                    }
                }
            });
        }
    }
    for glyph in &glyphs {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            glyph.draw(|px, py, v| {
                let x = px + bounds.min.x as u32;
                let y = py + bounds.min.y as u32;
                let old_p = im.get_pixel_mut(x, y);
                let new_p = Rgba {
                    data: [255, 255, 255, (v * 255.0) as u8],
                };
                old_p.blend(&new_p);
            });
        }
    }

    im
}

fn blend_copy(im1: &mut RgbaImage, im2: &RgbaImage, target_x: u32, target_y: u32) {
    for (x, y, p2) in im2.enumerate_pixels() {
        let p1 = im1.get_pixel_mut(target_x + x, target_y + y);
        p1.blend(p2);
    }
}

fn adjust_target(im: RgbaImage, target_width: u32) -> RgbaImage {
    if im.width() < target_width {
        let mut out = DynamicImage::new_rgba8(target_width, im.height()).to_rgba();
        out.copy_from(&im, (target_width - im.width()) / 2, 0);
        out
    } else if im.width() > target_width {
        let dim = DynamicImage::ImageRgba8(im);
        dim.resize(target_width, dim.height(), image::FilterType::CatmullRom)
            .to_rgba()
    } else {
        im
    }
}

fn draw_on_image(
    im: &DynamicImage,
    font: &Font,
    top_text: &str,
    bottom_text: &str,
) -> DynamicImage {
    let top = adjust_target(render_text(font, top_text), im.width());
    let bottom = adjust_target(render_text(font, bottom_text), im.width());
    let mut buf = im.to_rgba();
    blend_copy(&mut buf, &top, 0, OUTER_MARGIN);
    blend_copy(
        &mut buf,
        &bottom,
        0,
        im.height() - bottom.height() - OUTER_MARGIN,
    );
    DynamicImage::ImageRgba8(buf)
}
