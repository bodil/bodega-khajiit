use egg_mode::{
    self,
    tweet::{user_timeline, Timeline, Tweet},
    KeyPair, Token,
};
use image::{self, DynamicImage};
use rusttype::Font;
use state::State;
use std::env;
use tokio_core::reactor::{Core, Timeout};

use http;
use render;
use web;

use config::{ALT_TEXT, BOTTOM_TEXT, FONT, INTERVAL, TIMELINE_PAGE_SIZE, TOP_TEXT, TWITTER_HANDLE};

pub fn main() -> Result<(), String> {
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
    web::run_server(&mut core)?;
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
    let timeline = user_timeline(TWITTER_HANDLE, false, false, token, &handle)
        .with_page_size(TIMELINE_PAGE_SIZE);
    let (_timeline, images) = check_timeline(core, state, timeline)?;
    if env::var("DRY_RUN").is_err() {
        for image_url in images {
            let img = process_image(core, &image_url, |i| {
                render::draw_on_image(&i, font, TOP_TEXT, BOTTOM_TEXT)
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

fn send_tweet(core: &mut Core, token: &Token, img: Vec<u8>) -> Result<(), String> {
    use egg_mode::media::{media_types::image_jpg, UploadBuilder};
    use egg_mode::tweet::DraftTweet;

    info!("Tweeting image of size {}", img.len());

    let handle = core.handle();
    match core.run(
        UploadBuilder::new(img, image_jpg())
            .alt_text(ALT_TEXT)
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

fn process_image<F>(core: &mut Core, url: &str, f: F) -> Result<Vec<u8>, String>
where
    F: Fn(DynamicImage) -> DynamicImage,
{
    let image_data = http::load_url(core, url)?;
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
