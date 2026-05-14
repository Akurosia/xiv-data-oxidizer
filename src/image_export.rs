use ironworks::Ironworks;
use ironworks::excel::Language;
use ironworks::file::tex::{Format, Texture};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use crate::export::language_code;

const WEBP_MAX_DIMENSION: u32 = 16383;
const WEBP_QUALITY: f32 = 78.0;
const WEBP_METHOD: i32 = 4;
const WEBP_ALPHA_QUALITY: i32 = 80;

pub struct TextureImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct ImageExporter {
    prefer_high_res: bool,
    seen_paths: HashSet<String>,
}

impl ImageExporter {
    pub fn new(prefer_high_res: bool) -> Self {
        Self {
            prefer_high_res,
            seen_paths: HashSet::new(),
        }
    }

    pub fn export_icons(
        &mut self,
        ironworks: &Ironworks,
        language: Language,
        icon_ids: &Vec<u32>,
    ) -> Result<usize, Box<dyn Error>> {
        let mut count = 0;

        for icon_id in icon_ids {
            if let Some(path) = self.resolve_icon_path(ironworks, language, *icon_id) {
                if self.seen_paths.insert(path.clone()) {
                    export_texture(ironworks, &path)?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    fn resolve_icon_path(
        &self,
        ironworks: &Ironworks,
        language: Language,
        icon_id: u32,
    ) -> Option<String> {
        icon_candidates(language, icon_id, self.prefer_high_res)
            .into_iter()
            .find(|path| ironworks.file::<Texture>(path).is_ok())
    }
}

fn icon_candidates(language: Language, icon_id: u32, prefer_high_res: bool) -> Vec<String> {
    let mut candidates = Vec::new();
    let folder = icon_id / 1000;
    let language = language_code(&language);
    let language_prefix = match language {
        "??" => String::new(),
        code => format!("{}/", code),
    };

    if prefer_high_res {
        candidates.push(format!(
            "ui/icon/{folder:03}000/{language_prefix}{icon_id:06}_hr1.tex"
        ));
        candidates.push(format!("ui/icon/{folder:03}000/{icon_id:06}_hr1.tex"));
    }

    candidates.push(format!(
        "ui/icon/{folder:03}000/{language_prefix}{icon_id:06}.tex"
    ));
    candidates.push(format!("ui/icon/{folder:03}000/{icon_id:06}.tex"));

    candidates
}

pub fn export_texture(ironworks: &Ironworks, source_path: &str) -> Result<(), Box<dyn Error>> {
    let output_path = output_path_for_texture(source_path);
    export_texture_to(ironworks, source_path, &output_path)
}

pub fn export_texture_to(
    ironworks: &Ironworks,
    source_path: &str,
    output_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let image = load_texture_image(ironworks, source_path)?;

    save_rgba_webp(output_path, image.width, image.height, &image.rgba)
}

pub fn load_texture_image(
    ironworks: &Ironworks,
    source_path: &str,
) -> Result<TextureImage, Box<dyn Error>> {
    let texture = ironworks.file::<Texture>(source_path)?;
    let rgba = texture_to_rgba(&texture)?;

    Ok(TextureImage {
        width: u32::from(texture.width()),
        height: u32::from(texture.height()),
        rgba,
    })
}

pub fn save_rgba_webp(
    output_path: &PathBuf,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let image = resize_for_webp_if_needed(width, height, rgba)?;
    let has_transparency = image.rgba.chunks_exact(4).any(|pixel| pixel[3] < u8::MAX);
    let mut config = webp::WebPConfig::new()
        .map_err(|_| "Failed to create WebP encoder config.")?;
    config.lossless = 0;
    config.method = WEBP_METHOD;
    config.quality = WEBP_QUALITY;
    config.alpha_compression = 1;
    config.alpha_quality = WEBP_ALPHA_QUALITY;

    let encoded = if has_transparency {
        webp::Encoder::from_rgba(&image.rgba, image.width, image.height)
            .encode_advanced(&config)
            .map_err(|err| format!("Failed to encode WebP: {:?}", err))?
    } else {
        let rgb = rgba_to_rgb(&image.rgba);
        webp::Encoder::from_rgb(&rgb, image.width, image.height)
            .encode_advanced(&config)
            .map_err(|err| format!("Failed to encode WebP: {:?}", err))?
    };

    fs::write(output_path, encoded.as_ref())?;

    Ok(())
}

fn resize_for_webp_if_needed(
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<TextureImage, Box<dyn Error>> {
    if width <= WEBP_MAX_DIMENSION && height <= WEBP_MAX_DIMENSION {
        return Ok(TextureImage {
            width,
            height,
            rgba: rgba.to_vec(),
        });
    }

    let scale = f64::min(
        f64::from(WEBP_MAX_DIMENSION) / f64::from(width),
        f64::from(WEBP_MAX_DIMENSION) / f64::from(height),
    );
    let resized_width = u32::max(1, (f64::from(width) * scale).floor() as u32);
    let resized_height = u32::max(1, (f64::from(height) * scale).floor() as u32);
    let image = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or("Failed to build RGBA image for WebP resize.")?;
    let resized = image::imageops::resize(
        &image,
        resized_width,
        resized_height,
        image::imageops::FilterType::Lanczos3,
    );

    Ok(TextureImage {
        width: resized_width,
        height: resized_height,
        rgba: resized.into_raw(),
    })
}

fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);

    for pixel in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&pixel[..3]);
    }

    rgb
}

fn output_path_for_texture(source_path: &str) -> PathBuf {
    let mut path = PathBuf::from("output/images");

    for part in source_path.split('/') {
        path.push(part);
    }

    path.set_extension("webp");
    path
}

fn texture_to_rgba(texture: &Texture) -> Result<Vec<u8>, Box<dyn Error>> {
    let width = usize::from(texture.width());
    let height = usize::from(texture.height());

    match texture.format() {
        Format::L8Unorm => Ok(l8_to_rgba(texture.data(), width, height)),
        Format::A8Unorm => Ok(a8_or_bgra_to_rgba(texture.data(), width, height)),
        Format::Bgra8Unorm | Format::Rgba8Unknown => Ok(bgra_to_rgba(texture.data(), width, height, true)),
        Format::Bgrx8Unorm => Ok(bgra_to_rgba(texture.data(), width, height, false)),
        Format::Bgra4Unorm => Ok(bgra4_to_rgba(texture.data(), width, height)),
        Format::Bgr5a1Unorm => Ok(bgr5a1_to_rgba(texture.data(), width, height)),
        Format::Bc1Unorm => decode_block_texture(texture2ddecoder::decode_bc1, texture),
        Format::Bc2Unorm => decode_block_texture(texture2ddecoder::decode_bc2, texture),
        Format::Bc3Unorm => decode_block_texture(texture2ddecoder::decode_bc3, texture),
        Format::Bc5Unorm => decode_block_texture(texture2ddecoder::decode_bc5, texture),
        Format::Bc7Unorm => decode_block_texture(texture2ddecoder::decode_bc7, texture),
        format => Err(format!("Unsupported texture format: {:?}", format).into()),
    }
}

fn decode_block_texture(
    decoder: fn(&[u8], usize, usize, &mut [u32]) -> Result<(), &'static str>,
    texture: &Texture,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let width = usize::from(texture.width());
    let height = usize::from(texture.height());
    let mut image = vec![0; width * height];
    decoder(texture.data(), width, height, &mut image)?;

    let mut rgba = Vec::with_capacity(width * height * 4);
    for pixel in image {
        let [b, g, r, a] = pixel.to_le_bytes();
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    Ok(rgba)
}

fn l8_to_rgba(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);

    for value in data.iter().take(width * height) {
        rgba.extend_from_slice(&[*value, *value, *value, 0xff]);
    }

    rgba
}

fn a8_or_bgra_to_rgba(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    if data.len() >= width * height * 4 {
        return bgra_to_rgba(data, width, height, true);
    }

    let mut rgba = Vec::with_capacity(width * height * 4);

    for alpha in data.iter().take(width * height) {
        rgba.extend_from_slice(&[0xff, 0xff, 0xff, *alpha]);
    }

    rgba
}

fn bgra_to_rgba(data: &[u8], width: usize, height: usize, has_alpha: bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);

    for pixel in data.chunks_exact(4).take(width * height) {
        let alpha = match has_alpha {
            true => pixel[3],
            false => 0xff,
        };
        rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], alpha]);
    }

    rgba
}

fn bgra4_to_rgba(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);

    for pixel in data.chunks_exact(2).take(width * height) {
        let value = u16::from_le_bytes([pixel[0], pixel[1]]);
        let b = expand_4_bit((value & 0x000f) as u8);
        let g = expand_4_bit(((value >> 4) & 0x000f) as u8);
        let r = expand_4_bit(((value >> 8) & 0x000f) as u8);
        let a = expand_4_bit(((value >> 12) & 0x000f) as u8);
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    rgba
}

fn bgr5a1_to_rgba(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(width * height * 4);

    for pixel in data.chunks_exact(2).take(width * height) {
        let value = u16::from_le_bytes([pixel[0], pixel[1]]);
        let b = expand_5_bit((value & 0x001f) as u8);
        let g = expand_5_bit(((value >> 5) & 0x001f) as u8);
        let r = expand_5_bit(((value >> 10) & 0x001f) as u8);
        let a = match value & 0x8000 {
            0 => 0x00,
            _ => 0xff,
        };
        rgba.extend_from_slice(&[r, g, b, a]);
    }

    rgba
}

fn expand_4_bit(value: u8) -> u8 {
    (value << 4) | value
}

fn expand_5_bit(value: u8) -> u8 {
    (value << 3) | (value >> 2)
}
