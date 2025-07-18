use std::cmp::max;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

use crate::audio::Audio;
use crate::channel::{Channel, SharedChannel};
use crate::graphics::Graphics;
use crate::image::{Color, Image, Rgb24, SharedImage};
use crate::input::Input;
use crate::keys::Key;
use crate::music::{Music, SharedMusic};
use crate::resource::Resource;
use crate::settings::{
    CURSOR_DATA, CURSOR_HEIGHT, CURSOR_WIDTH, DEFAULT_COLORS, DEFAULT_FPS, DEFAULT_QUIT_KEY,
    DEFAULT_TITLE, DEFAULT_TONE_0, DEFAULT_TONE_1, DEFAULT_TONE_2, DEFAULT_TONE_3, DISPLAY_RATIO,
    FONT_DATA, FONT_HEIGHT, FONT_WIDTH, ICON_COLKEY, ICON_DATA, ICON_SCALE, IMAGE_SIZE,
    NUM_CHANNELS, NUM_FONT_ROWS, NUM_IMAGES, NUM_MUSICS, NUM_SOUNDS, NUM_TILEMAPS, NUM_TONES,
    TILEMAP_SIZE,
};
use crate::sound::{SharedSound, Sound};
use crate::system::System;
use crate::tilemap::{ImageSource, SharedTilemap, Tilemap};
use crate::tone::{SharedTone, Tone};

static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub static COLORS: LazyLock<shared_type!(Vec<Rgb24>)> =
    LazyLock::new(|| new_shared_type!(DEFAULT_COLORS.to_vec()));

pub static IMAGES: LazyLock<shared_type!(Vec<SharedImage>)> = LazyLock::new(|| {
    new_shared_type!((0..NUM_IMAGES)
        .map(|_| Image::new(IMAGE_SIZE, IMAGE_SIZE))
        .collect())
});

static TILEMAPS: LazyLock<shared_type!(Vec<SharedTilemap>)> = LazyLock::new(|| {
    new_shared_type!((0..NUM_TILEMAPS)
        .map(|_| Tilemap::new(TILEMAP_SIZE, TILEMAP_SIZE, ImageSource::Index(0)))
        .collect())
});

static CURSOR_IMAGE: LazyLock<SharedImage> = LazyLock::new(|| {
    let image = Image::new(CURSOR_WIDTH, CURSOR_HEIGHT);
    image.lock().set(0, 0, &CURSOR_DATA);
    image
});

pub static FONT_IMAGE: LazyLock<SharedImage> = LazyLock::new(|| {
    let width = FONT_WIDTH * NUM_FONT_ROWS;
    let height = FONT_HEIGHT * (FONT_DATA.len() as u32).div_ceil(NUM_FONT_ROWS);
    let image = Image::new(width, height);
    {
        let mut image = image.lock();
        for (fi, data) in FONT_DATA.iter().enumerate() {
            let row = fi as u32 / NUM_FONT_ROWS;
            let col = fi as u32 % NUM_FONT_ROWS;
            let mut data = *data;
            for yi in 0..FONT_HEIGHT {
                for xi in 0..FONT_WIDTH {
                    let x = FONT_WIDTH * col + xi;
                    let y = FONT_HEIGHT * row + yi;
                    let color = Color::from((data & 0x800000) != 0);
                    image.canvas.write_data(x as usize, y as usize, color);
                    data <<= 1;
                }
            }
        }
    }
    image
});

pub static CHANNELS: LazyLock<shared_type!(Vec<SharedChannel>)> =
    LazyLock::new(|| new_shared_type!((0..NUM_CHANNELS).map(|_| Channel::new()).collect()));

macro_rules! set_default_tone {
    ($tone:ident, $default_tone:ident) => {{
        $tone.mode = $default_tone.0;
        $tone.sample_bits = $default_tone.1;
        $tone.wavetable = $default_tone.2.to_vec();
        $tone.gain = $default_tone.3;
    }};
}

pub static TONES: LazyLock<shared_type!(Vec<SharedTone>)> = LazyLock::new(|| {
    new_shared_type!((0..NUM_TONES)
        .map(|index| {
            let tone = Tone::new();
            {
                let mut tone = tone.lock();
                {
                    match index {
                        0 => set_default_tone!(tone, DEFAULT_TONE_0),
                        1 => set_default_tone!(tone, DEFAULT_TONE_1),
                        2 => set_default_tone!(tone, DEFAULT_TONE_2),
                        3 => set_default_tone!(tone, DEFAULT_TONE_3),
                        _ => panic!(),
                    }
                }
            }
            tone
        })
        .collect())
});

pub static SOUNDS: LazyLock<shared_type!(Vec<SharedSound>)> =
    LazyLock::new(|| new_shared_type!((0..NUM_SOUNDS).map(|_| Sound::new()).collect()));

static MUSICS: LazyLock<shared_type!(Vec<SharedMusic>)> =
    LazyLock::new(|| new_shared_type!((0..NUM_MUSICS).map(|_| Music::new()).collect()));

pub struct Pyxel {
    // System
    pub(crate) system: System,
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,

    // Resource
    pub(crate) resource: Resource,

    // Input
    pub(crate) input: Input,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_wheel: i32,
    pub input_keys: Vec<Key>,
    pub input_text: String,
    pub dropped_files: Vec<String>,

    // Graphics
    pub(crate) graphics: Graphics,
    pub colors: shared_type!(Vec<Rgb24>),
    pub images: shared_type!(Vec<SharedImage>),
    pub tilemaps: shared_type!(Vec<SharedTilemap>),
    pub screen: SharedImage,
    pub cursor: SharedImage,
    pub font: SharedImage,

    // Audio
    pub channels: shared_type!(Vec<SharedChannel>),
    pub tones: shared_type!(Vec<SharedTone>),
    pub sounds: shared_type!(Vec<SharedSound>),
    pub musics: shared_type!(Vec<SharedMusic>),
}

pub fn init(
    width: u32,
    height: u32,
    title: Option<&str>,
    fps: Option<u32>,
    quit_key: Option<Key>,
    display_scale: Option<u32>,
    capture_scale: Option<u32>,
    capture_sec: Option<u32>,
) -> Pyxel {
    assert!(
        !IS_INITIALIZED.swap(true, Ordering::Relaxed),
        "Pyxel already initialized"
    );

    // Default parameters
    let title = title.unwrap_or(DEFAULT_TITLE);
    let quit_key = quit_key.unwrap_or(DEFAULT_QUIT_KEY);
    let fps = fps.unwrap_or(DEFAULT_FPS);

    // Platform
    pyxel_platform::init(|display_width, display_height| {
        let display_scale = max(
            display_scale.map_or_else(
                || {
                    (f32::min(
                        display_width as f32 / width as f32,
                        display_height as f32 / height as f32,
                    ) * DISPLAY_RATIO) as u32
                },
                |display_scale| display_scale,
            ),
            1,
        );
        (title, width * display_scale, height * display_scale)
    });

    // System
    let system = System::new(fps, quit_key);
    let frame_count = 0;

    // Resource
    let resource = Resource::new(capture_scale, capture_sec, fps);

    // Input
    let input = Input::new();
    let mouse_x = 0;
    let mouse_y = 0;
    let mouse_wheel = 0;
    let input_keys = Vec::new();
    let input_text = String::new();
    let dropped_files = Vec::new();

    // Graphics
    let graphics = Graphics::new();
    let colors = COLORS.clone();
    let images = IMAGES.clone();
    let tilemaps = TILEMAPS.clone();
    let screen = Image::new(width, height);
    let cursor = CURSOR_IMAGE.clone();
    let font = FONT_IMAGE.clone();

    // Audio
    let _ = Audio::new();
    let channels = CHANNELS.clone();
    let tones = TONES.clone();
    let sounds = SOUNDS.clone();
    let musics = MUSICS.clone();

    let pyxel = Pyxel {
        // System
        system,
        width,
        height,
        frame_count,

        // Resource
        resource,

        // Input
        input,
        mouse_x,
        mouse_y,
        mouse_wheel,
        input_keys,
        input_text,
        dropped_files,

        // Graphics
        graphics,
        colors,
        images,
        tilemaps,
        screen,
        cursor,
        font,

        // Audio
        channels,
        tones,
        sounds,
        musics,
    };

    pyxel.icon(&ICON_DATA, ICON_SCALE, ICON_COLKEY);
    pyxel
}
