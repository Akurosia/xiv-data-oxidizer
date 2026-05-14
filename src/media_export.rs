use ironworks::excel::{Excel, Field};
use ironworks::file::exh::ColumnDefinition;
use ironworks::Ironworks;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::exd_schema::field_names;
use crate::image_export::{export_texture_to, load_texture_image, save_rgba_webp};

const DEFAULT_PATH_LIST_URL: &str = "https://rl2.perchbird.dev/download/PathList.gz";
const PATH_LIST_CACHE_TIME: Duration = Duration::from_secs(24 * 60 * 60);

const SCD_OGG_XOR_TABLE: [u8; 256] = [
    0x3A, 0x32, 0x32, 0x32, 0x03, 0x7E, 0x12, 0xF7, 0xB2, 0xE2, 0xA2, 0x67, 0x32, 0x32, 0x22,
    0x32, 0x32, 0x52, 0x16, 0x1B, 0x3C, 0xA1, 0x54, 0x7B, 0x1B, 0x97, 0xA6, 0x93, 0x1A, 0x4B,
    0xAA, 0xA6, 0x7A, 0x7B, 0x1B, 0x97, 0xA6, 0xF7, 0x02, 0xBB, 0xAA, 0xA6, 0xBB, 0xF7, 0x2A,
    0x51, 0xBE, 0x03, 0xF4, 0x2A, 0x51, 0xBE, 0x03, 0xF4, 0x2A, 0x51, 0xBE, 0x12, 0x06, 0x56,
    0x27, 0x32, 0x32, 0x36, 0x32, 0xB2, 0x1A, 0x3B, 0xBC, 0x91, 0xD4, 0x7B, 0x58, 0xFC, 0x0B,
    0x55, 0x2A, 0x15, 0xBC, 0x40, 0x92, 0x0B, 0x5B, 0x7C, 0x0A, 0x95, 0x12, 0x35, 0xB8, 0x63,
    0xD2, 0x0B, 0x3B, 0xF0, 0xC7, 0x14, 0x51, 0x5C, 0x94, 0x86, 0x94, 0x59, 0x5C, 0xFC, 0x1B,
    0x17, 0x3A, 0x3F, 0x6B, 0x37, 0x32, 0x32, 0x30, 0x32, 0x72, 0x7A, 0x13, 0xB7, 0x26, 0x60,
    0x7A, 0x13, 0xB7, 0x26, 0x50, 0xBA, 0x13, 0xB4, 0x2A, 0x50, 0xBA, 0x13, 0xB5, 0x2E, 0x40,
    0xFA, 0x13, 0x95, 0xAE, 0x40, 0x38, 0x18, 0x9A, 0x92, 0xB0, 0x38, 0x00, 0xFA, 0x12, 0xB1,
    0x7E, 0x00, 0xDB, 0x96, 0xA1, 0x7C, 0x08, 0xDB, 0x9A, 0x91, 0xBC, 0x08, 0xD8, 0x1A, 0x86,
    0xE2, 0x70, 0x39, 0x1F, 0x86, 0xE0, 0x78, 0x7E, 0x03, 0xE7, 0x64, 0x51, 0x9C, 0x8F, 0x34,
    0x6F, 0x4E, 0x41, 0xFC, 0x0B, 0xD5, 0xAE, 0x41, 0xFC, 0x0B, 0xD5, 0xAE, 0x41, 0xFC, 0x3B,
    0x70, 0x71, 0x64, 0x33, 0x32, 0x12, 0x32, 0x32, 0x36, 0x70, 0x34, 0x2B, 0x56, 0x22, 0x70,
    0x3A, 0x13, 0xB7, 0x26, 0x60, 0xBA, 0x1B, 0x94, 0xAA, 0x40, 0x38, 0x00, 0xFA, 0xB2, 0xE2,
    0xA2, 0x67, 0x32, 0x32, 0x12, 0x32, 0xB2, 0x32, 0x32, 0x32, 0x32, 0x75, 0xA3, 0x26, 0x7B,
    0x83, 0x26, 0xF9, 0x83, 0x2E, 0xFF, 0xE3, 0x16, 0x7D, 0xC0, 0x1E, 0x63, 0x21, 0x07, 0xE3,
    0x01,
];

pub fn export_bgm(ironworks: &Ironworks, excel: &Excel) -> Result<usize, Box<dyn Error>> {
    let mut exported = 0;
    let mut seen = HashSet::new();

    for file_path in bgm_paths(excel)? {
        if seen.insert(file_path.clone()) {
            match export_scd_file(ironworks, &file_path) {
                Ok(count) => exported += count,
                Err(err) => eprintln!("Failed to export BGM {}. {}", file_path, err),
            }
        }
    }

    Ok(exported)
}

pub fn export_uld(
    ironworks: &Ironworks,
    path_list: Option<&Path>,
) -> Result<usize, Box<dyn Error>> {
    let cache_path;
    let path_list = match path_list {
        Some(path) => path,
        None => {
            cache_path = get_cached_path_list()?;
            cache_path.as_path()
        }
    };
    let mut exported = 0;
    let mut seen = HashSet::new();

    for game_path in load_uld_paths(path_list)? {
        if !seen.insert(game_path.clone()) {
            continue;
        }

        let relative_path = strip_texture_extension(&game_path)
            .strip_prefix("ui/uld/")
            .unwrap_or(strip_texture_extension(&game_path));
        let output_path = output_path("output/uld", relative_path);
        match export_texture_to(ironworks, &game_path, &output_path.with_extension("webp")) {
            Ok(_) => exported += 1,
            Err(err) => eprintln!("Failed to export ULD texture {}. {}", game_path, err),
        }
    }

    Ok(exported)
}

pub fn export_loading_images(ironworks: &Ironworks) -> Result<usize, Box<dyn Error>> {
    let mut exported = 0;

    for index in 1.. {
        let base_path = format!("ui/loadingimage/-nowloading_base{index:02}.tex");
        if ironworks.file::<ironworks::file::tex::Texture>(&base_path).is_err() {
            break;
        }

        export_texture_to(ironworks, &base_path, &loading_image_output_path(&base_path))?;
        exported += 1;

        let hr_path = format!("ui/loadingimage/-nowloading_base{index:02}_hr1.tex");
        if ironworks.file::<ironworks::file::tex::Texture>(&hr_path).is_ok() {
            export_texture_to(ironworks, &hr_path, &loading_image_output_path(&hr_path))?;
            exported += 1;
        }
    }

    Ok(exported)
}

pub fn export_maps(ironworks: &Ironworks, excel: &Excel) -> Result<usize, Box<dyn Error>> {
    let place_names = place_name_lookup(excel)?;
    let rows = sheet_rows(excel, "Map")?;
    let mut exported = 0;
    let mut used_names = HashMap::new();

    for row in rows {
        let map_id = match row.get("Id") {
            Some(value) if !value.is_empty() => value,
            _ => continue,
        };
        let place_name_id = row
            .get("PlaceName")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or_default();
        if place_name_id == 0 {
            continue;
        }

        let image = match load_map_image(ironworks, map_id) {
            Ok(image) => image,
            Err(_) => continue,
        };

        let folder = map_id_folder(map_id);
        let place_name = place_names
            .get(&place_name_id)
            .map(String::as_str)
            .unwrap_or(map_id);
        let key = format!("{folder}/{}", strip_map_markers(place_name));
        let index = used_names.entry(key.clone()).or_insert(0);
        let suffix = match *index {
            0 => String::new(),
            value => format!(" - {value}"),
        };
        *index += 1;

        let file_name = sanitize_file_name(&format!("{}{}", strip_map_markers(place_name), suffix));
        let output = PathBuf::from("output/maps")
            .join(folder)
            .join(file_name)
            .with_extension("webp");
        save_rgba_webp(&output, image.width, image.height, &image.rgba)?;
        exported += 1;
    }

    Ok(exported)
}

fn bgm_paths(excel: &Excel) -> Result<Vec<String>, Box<dyn Error>> {
    let mut paths = Vec::new();

    collect_file_column_paths(excel, "BGM", "File", &mut paths)?;
    collect_file_column_paths(excel, "OrchestrionPath", "File", &mut paths)?;

    Ok(paths)
}

fn collect_file_column_paths(
    excel: &Excel,
    sheet_name: &str,
    field_name: &str,
    paths: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let sheet = excel.sheet(sheet_name)?;
    let headers = match field_names(sheet_name)? {
        Some(headers) => headers,
        None => return Ok(()),
    };
    let field_index = match headers.iter().position(|header| header == field_name) {
        Some(index) if index > 0 => index - 1,
        _ => return Ok(()),
    };

    let mut columns = sheet.columns()?;
    columns.sort_by_key(|column| column.offset);
    let column = match columns.get(field_index) {
        Some(column) => column,
        None => return Ok(()),
    };
    let specifier = ColumnDefinition {
        kind: column.kind,
        offset: column.offset,
    };

    for row in sheet.into_iter() {
        let row = row?;
        if let Field::String(value) = row.field(&specifier)? {
            let path = value.to_string();
            if !path.trim().is_empty() {
                paths.push(path);
            }
        }
    }

    Ok(())
}

fn sheet_rows(excel: &Excel, sheet_name: &str) -> Result<Vec<HashMap<String, String>>, Box<dyn Error>> {
    let sheet = excel.sheet(sheet_name)?;
    let headers = match field_names(sheet_name)? {
        Some(headers) => headers,
        None => return Ok(Vec::new()),
    };
    let mut columns = sheet.columns()?;
    columns.sort_by_key(|column| column.offset);
    let mut rows = Vec::new();

    for row in sheet.into_iter() {
        let row = row?;
        let mut values = HashMap::new();
        values.insert("#".to_string(), row.row_id().to_string());

        for (index, column) in columns.iter().enumerate() {
            let Some(header) = headers.get(index + 1) else {
                continue;
            };
            let specifier = ColumnDefinition {
                kind: column.kind,
                offset: column.offset,
            };
            values.insert(header.clone(), field_to_plain_string(row.field(&specifier)?));
        }

        rows.push(values);
    }

    Ok(rows)
}

fn place_name_lookup(excel: &Excel) -> Result<HashMap<u32, String>, Box<dyn Error>> {
    let mut lookup = HashMap::new();

    for row in sheet_rows(excel, "PlaceName")? {
        let id = row.get("#").and_then(|value| value.parse::<u32>().ok());
        let name = row.get("Name");
        if let (Some(id), Some(name)) = (id, name) {
            lookup.insert(id, name.clone());
        }
    }

    Ok(lookup)
}

fn field_to_plain_string(field: Field) -> String {
    match field {
        Field::String(value) => value.to_string(),
        Field::Bool(value) => {
            if value {
                String::from("True")
            } else {
                String::from("False")
            }
        }
        Field::I8(value) => value.to_string(),
        Field::I16(value) => value.to_string(),
        Field::I32(value) => value.to_string(),
        Field::I64(value) => value.to_string(),
        Field::U8(value) => value.to_string(),
        Field::U16(value) => value.to_string(),
        Field::U32(value) => value.to_string(),
        Field::U64(value) => value.to_string(),
        Field::F32(value) => value.to_string(),
    }
}

fn load_map_image(
    ironworks: &Ironworks,
    map_id: &str,
) -> Result<crate::image_export::TextureImage, Box<dyn Error>> {
    let file_name = map_id.replace('/', "");
    let image_path = format!("ui/map/{map_id}/{file_name}_m.tex");
    let mask_path = format!("ui/map/{map_id}/{file_name}m_m.tex");
    let mut image = load_texture_image(ironworks, &image_path)?;

    if let Ok(mask) = load_texture_image(ironworks, &mask_path) {
        if mask.width == image.width && mask.height == image.height {
            multiply_blend_rgba(&mut image.rgba, &mask.rgba);
        }
    }

    Ok(image)
}

fn multiply_blend_rgba(image: &mut [u8], mask: &[u8]) {
    for (pixel, mask_pixel) in image.chunks_exact_mut(4).zip(mask.chunks_exact(4)) {
        if mask_pixel[3] != 0 {
            pixel[0] = ((u16::from(pixel[0]) * u16::from(mask_pixel[0])) / 255) as u8;
            pixel[1] = ((u16::from(pixel[1]) * u16::from(mask_pixel[1])) / 255) as u8;
            pixel[2] = ((u16::from(pixel[2]) * u16::from(mask_pixel[2])) / 255) as u8;
        }
    }
}

fn export_scd_file(ironworks: &Ironworks, game_path: &str) -> Result<usize, Box<dyn Error>> {
    let data = ironworks.file::<Vec<u8>>(game_path)?;
    let entries = decode_scd_entries(&data)?;
    let mut exported = 0;

    for (index, entry) in entries.into_iter().enumerate() {
        let stem = Path::new(game_path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("bgm");
        let name = match index {
            0 => sanitize_file_name(stem),
            _ => sanitize_file_name(&format!("{}-{}", stem, index + 1)),
        };
        let target_dir = Path::new("output/bgm").join(Path::new(game_path).parent().unwrap_or(Path::new("")));
        let target = target_dir.join(format!("{}.{}", name, entry.extension));

        fs::create_dir_all(&target_dir)?;
        fs::write(target, entry.data)?;
        exported += 1;
    }

    Ok(exported)
}

struct ScdEntry {
    extension: &'static str,
    data: Vec<u8>,
}

#[derive(Clone, Copy)]
struct ScdEntryHeader {
    data_size: usize,
    codec: i32,
    samples_offset: usize,
    aux_chunk_count: usize,
}

fn decode_scd_entries(data: &[u8]) -> Result<Vec<ScdEntry>, Box<dyn Error>> {
    if read_i64(data, 0, false)? != 0x5345444253534346 {
        return Err("Invalid SCD magic.".into());
    }

    let little_endian = match (read_i32(data, 8, false)?, read_i32(data, 8, true)?) {
        (2 | 3, _) => false,
        (_, 2 | 3) => true,
        _ => return Err("Invalid SCD version.".into()),
    };

    let header_offset = read_i16(data, 0x0e, little_endian)? as usize;
    let entry_count = read_i16(data, header_offset + 0x04, little_endian)? as usize;
    let entry_table_offset = read_i32(data, header_offset + 0x0c, little_endian)? as usize;
    let mut entries = Vec::new();

    for index in 0..entry_count {
        let entry_header_offset = read_i32(data, entry_table_offset + 4 * index, little_endian)? as usize;
        let header = read_scd_entry_header(data, entry_header_offset, little_endian)?;
        if header.data_size == 0 || header.codec == 0 {
            continue;
        }

        let chunks_offset = entry_header_offset + 0x20;
        let mut data_offset = chunks_offset;
        for _ in 0..header.aux_chunk_count {
            data_offset += read_i32(data, data_offset + 4, little_endian)? as usize;
        }

        entries.push(match header.codec {
            0x06 => ScdEntry {
                extension: "ogg",
                data: decode_scd_ogg(data, data_offset, header, little_endian)?,
            },
            0x0c => ScdEntry {
                extension: "wav",
                data: decode_scd_adpcm(data, chunks_offset, data_offset, header)?,
            },
            _ => return Err(format!("Unsupported SCD codec: {}", header.codec).into()),
        });
    }

    Ok(entries)
}

fn read_scd_entry_header(
    data: &[u8],
    offset: usize,
    little_endian: bool,
) -> Result<ScdEntryHeader, Box<dyn Error>> {
    Ok(ScdEntryHeader {
        data_size: read_i32(data, offset, little_endian)? as usize,
        codec: read_i32(data, offset + 0x0c, little_endian)?,
        samples_offset: read_i32(data, offset + 0x18, little_endian)? as usize,
        aux_chunk_count: read_i16(data, offset + 0x1c, little_endian)? as usize,
    })
}

fn decode_scd_ogg(
    data: &[u8],
    data_offset: usize,
    header: ScdEntryHeader,
    little_endian: bool,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let crypt_type = read_i16(data, data_offset, little_endian)?;
    if crypt_type != 0x0000 && crypt_type != 0x2002 && crypt_type != 0x2003 {
        return Err(format!("Unsupported OGG crypt type: {crypt_type:#06x}").into());
    }

    let seek_table_size = read_i32(data, data_offset + 0x10, little_endian)? as usize;
    let vorbis_header_size = read_i32(data, data_offset + 0x14, little_endian)? as usize;
    let vorbis_header_offset = data_offset + 0x20 + seek_table_size;
    let sound_data_offset = vorbis_header_offset + vorbis_header_size;
    let mut vorbis_header = data[vorbis_header_offset..vorbis_header_offset + vorbis_header_size].to_vec();

    if crypt_type == 0x2002 {
        let xor_value = data[data_offset + 0x02];
        if xor_value != 0 {
            for byte in &mut vorbis_header {
                *byte ^= xor_value;
            }
        }
    }

    let mut decoded = Vec::with_capacity(vorbis_header_size + header.data_size);
    decoded.extend_from_slice(&vorbis_header);
    decoded.extend_from_slice(&data[sound_data_offset..sound_data_offset + header.data_size]);

    if crypt_type == 0x2003 {
        let static_xor = (header.data_size & 0x7f) as u8;
        let table_offset = header.data_size & 0x3f;
        for (index, byte) in decoded.iter_mut().enumerate() {
            *byte ^= SCD_OGG_XOR_TABLE[(table_offset + index) & 0xff];
            *byte ^= static_xor;
        }
    }

    Ok(decoded)
}

fn decode_scd_adpcm(
    data: &[u8],
    chunks_offset: usize,
    data_offset: usize,
    header: ScdEntryHeader,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let wav_header_size = 0x10;
    let wav_header_offset = data_offset;
    let final_data_offset = chunks_offset + header.samples_offset;
    let mut decoded = Vec::with_capacity(0x1c + wav_header_size + header.data_size);

    decoded.extend_from_slice(b"RIFF");
    decoded.extend_from_slice(&((0x14 + wav_header_size + header.data_size) as u32).to_le_bytes());
    decoded.extend_from_slice(b"WAVEfmt ");
    decoded.extend_from_slice(&(wav_header_size as u32).to_le_bytes());
    decoded.extend_from_slice(&data[wav_header_offset..wav_header_offset + wav_header_size]);
    decoded.extend_from_slice(b"data");
    decoded.extend_from_slice(&(header.data_size as u32).to_le_bytes());
    decoded.extend_from_slice(&data[final_data_offset..final_data_offset + header.data_size]);

    Ok(decoded)
}

fn read_i16(data: &[u8], offset: usize, little_endian: bool) -> Result<i16, Box<dyn Error>> {
    let bytes: [u8; 2] = data[offset..offset + 2].try_into()?;
    Ok(match little_endian {
        true => i16::from_le_bytes(bytes),
        false => i16::from_be_bytes(bytes),
    })
}

fn read_i32(data: &[u8], offset: usize, little_endian: bool) -> Result<i32, Box<dyn Error>> {
    let bytes: [u8; 4] = data[offset..offset + 4].try_into()?;
    Ok(match little_endian {
        true => i32::from_le_bytes(bytes),
        false => i32::from_be_bytes(bytes),
    })
}

fn read_i64(data: &[u8], offset: usize, little_endian: bool) -> Result<i64, Box<dyn Error>> {
    let bytes: [u8; 8] = data[offset..offset + 8].try_into()?;
    Ok(match little_endian {
        true => i64::from_le_bytes(bytes),
        false => i64::from_be_bytes(bytes),
    })
}

fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .filter(|character| !r#"<>:"/\|?*"#.contains(*character))
        .collect()
}

fn load_uld_paths(path_list: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let file = fs::File::open(path_list)?;
    let extension = path_list
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if extension == "gz" {
        let decoder = flate2::read::GzDecoder::new(file);
        return load_uld_paths_from_reader(decoder);
    }

    load_uld_paths_from_reader(file)
}

fn get_cached_path_list() -> Result<PathBuf, Box<dyn Error>> {
    let path = PathBuf::from("output/cache/PathList.gz");

    if is_fresh_cache(&path)? {
        return Ok(path);
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let response = reqwest::blocking::get(DEFAULT_PATH_LIST_URL)?.error_for_status()?;
    let bytes = response.bytes()?;
    fs::write(&path, bytes)?;

    Ok(path)
}

fn is_fresh_cache(path: &Path) -> Result<bool, Box<dyn Error>> {
    if !path.exists() {
        return Ok(false);
    }

    let modified = fs::metadata(path)?.modified()?;
    let age = SystemTime::now().duration_since(modified)?;

    Ok(age < PATH_LIST_CACHE_TIME)
}

fn load_uld_paths_from_reader(reader: impl Read) -> Result<Vec<String>, Box<dyn Error>> {
    let mut paths = Vec::new();
    let reader = BufReader::new(reader);

    for line in reader.lines() {
        let line = line?;
        if let Some(path) = normalize_uld_line(&line) {
            paths.push(path);
        }
    }

    Ok(paths)
}

fn normalize_uld_line(line: &str) -> Option<String> {
    let mut line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    if let Some((_, path)) = line.split_once(',') {
        line = path.trim();
    }

    let path = line.trim_matches('"').replace('\\', "/").to_ascii_lowercase();
    if path.starts_with("ui/uld/") && (path.ends_with(".tex") || path.ends_with(".atex")) {
        Some(path)
    } else {
        None
    }
}

fn output_path(output_root: &str, game_path: &str) -> PathBuf {
    let mut path = PathBuf::from(output_root);

    for part in game_path.split('/') {
        path.push(part);
    }

    path
}

fn strip_texture_extension(path: &str) -> &str {
    path.strip_suffix(".atex")
        .or_else(|| path.strip_suffix(".tex"))
        .unwrap_or(path)
}

fn loading_image_output_path(game_path: &str) -> PathBuf {
    let relative_path = strip_texture_extension(game_path)
        .strip_prefix("ui/loadingimage/")
        .unwrap_or(strip_texture_extension(game_path));
    output_path("output/loadingimage", relative_path).with_extension("webp")
}

fn map_id_folder(map_id: &str) -> String {
    let first_segment = map_id.split('/').next().unwrap_or_default();
    if first_segment.eq_ignore_ascii_case("default") || first_segment.len() <= 3 {
        first_segment.to_string()
    } else {
        first_segment[..3].to_string()
    }
}

fn strip_map_markers(input: &str) -> String {
    input
        .replace("__Emphasis__", "")
        .replace("_Emphasis_", "")
        .replace("__Soft-Hyphen__", "")
        .replace("_Soft-Hyphen_", "")
        .replace("__SoftHyphen__", "")
        .replace("_SoftHyphen_", "")
}
