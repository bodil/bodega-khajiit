use std::time::Duration;

pub const TWITTER_HANDLE: &str = "Bodegacats_";

pub const TOP_TEXT: &str = "KHAJIIT HAS WARES";
pub const BOTTOM_TEXT: &str = "IF YOU HAVE COIN";
pub const ALT_TEXT: &str = "Khajiit has wares, if you have coin.";

pub const FONT: &[u8] = include_bytes!("../impact.ttf");
pub const FONT_SIZE: f32 = 128.0;
pub const BORDER_SIZE: u32 = 6;
pub const TEXT_MARGIN: f32 = 20.0;
pub const OUTER_MARGIN: u32 = 10;

pub const TIMELINE_PAGE_SIZE: i32 = 10;
pub const INTERVAL: Duration = Duration::from_secs(1800);
