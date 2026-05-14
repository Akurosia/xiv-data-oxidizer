use std::path::Path;
use std::{env, error::Error};

use ironworks::{
    Ironworks,
    excel::{Excel, Language},
    sqpack::{Install, SqPack},
};
mod exd_schema;
mod export;
mod formatter;
mod image_export;
mod media_export;

struct Options {
    format: export::OutputFormat,
    export_images: bool,
    high_res_images: bool,
    export_bgm: bool,
    export_maps: bool,
    export_loading_images: bool,
    uld_path_list: Option<String>,
    export_uld: bool,
    language: Option<Language>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!(
            "You must provide a game path. For example: cargo run -- \"C:\\Program Files (x86)\\Square Enix\\FINAL FANTASY XIV - A Realm Reborn\" --format csv"
        );
    }

    let path = Path::new(&args[1]);
    let options = parse_options(&args[2..])?;

    let ironworks = Ironworks::new().with_resource(SqPack::new(Install::at(path)));
    let image_ironworks = match options.export_images
        || options.export_uld
        || options.export_maps
        || options.export_loading_images
    {
        true => Some(Ironworks::new().with_resource(SqPack::new(Install::at(path)))),
        false => None,
    };
    let bgm_ironworks = match options.export_bgm {
        true => Some(Ironworks::new().with_resource(SqPack::new(Install::at(path)))),
        false => None,
    };
    let mut image_exporter = match options.export_images {
        true => Some(image_export::ImageExporter::new(options.high_res_images)),
        false => None,
    };
    let languages = export::available_languages(&ironworks);
    let export_languages = match options.language {
        Some(language) => vec![language],
        None => languages,
    };
    let media_language = options.language.unwrap_or(Language::English);
    let mut excel = Excel::new(ironworks);

    for language in export_languages {
        excel.set_default_language(language);
        let sheets = excel.list().expect("Could not retrieve sheet list.");

        println!(
            "Exporting {} sheets as {}",
            export::language_code(&language).to_uppercase(),
            options.format.extension().to_uppercase()
        );

        for sheet in sheets.iter() {
            match export::sheet(&excel, language, &sheet, options.format, options.export_images) {
                Ok(icon_ids) => {
                    if let (Some(ironworks), Some(exporter)) =
                        (image_ironworks.as_ref(), image_exporter.as_mut())
                    {
                        if let Err(err) = exporter.export_icons(ironworks, language, &icon_ids) {
                            eprintln!("Failed to export images for {}. {}", sheet, err);
                        }
                    }
                }
                // Log failed sheets and continue
                Err(err) => eprintln!("Failed to export {}. {}", sheet, err),
            }
        }
    }

    excel.set_default_language(media_language);

    if let Some(ironworks) = bgm_ironworks.as_ref() {
        match media_export::export_bgm(ironworks, &excel) {
            Ok(count) => println!("Exported {count} BGM files"),
            Err(err) => eprintln!("Failed to export BGM files. {}", err),
        }
    }

    if let Some(ironworks) = image_ironworks.as_ref() {
        if options.export_loading_images {
            match media_export::export_loading_images(ironworks) {
                Ok(count) => println!("Exported {count} loading images"),
                Err(err) => eprintln!("Failed to export loading images. {}", err),
            }
        }

        if options.export_maps {
            match media_export::export_maps(ironworks, &excel) {
                Ok(count) => println!("Exported {count} maps"),
                Err(err) => eprintln!("Failed to export maps. {}", err),
            }
        }
    }

    if options.export_uld {
        let path_list = options.uld_path_list.as_deref().map(Path::new);
        match media_export::export_uld(
            image_ironworks
                .as_ref()
                .expect("image ironworks should exist for ULD export"),
            path_list,
        ) {
            Ok(count) => println!("Exported {count} ULD textures"),
            Err(err) => eprintln!("Failed to export ULD textures. {}", err),
        }
    }

    // Quick debugging for schema updates

    // for language in languages {
    //     excel.set_default_language(language);
    //     export::sheet(&excel, language, &String::from("Mount"))?;
    // }

    // let language = Language::English;
    // excel.set_default_language(language);
    // export::sheet(&excel, language, "Mount")?;

    Ok(())
}

fn parse_options(args: &[String]) -> Result<Options, Box<dyn Error>> {
    let mut format = export::OutputFormat::Csv;
    let mut export_images = false;
    let mut high_res_images = false;
    let mut export_bgm = false;
    let mut export_maps = false;
    let mut export_loading_images = false;
    let mut uld_path_list = None;
    let mut export_uld = false;
    let mut language = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];

        match arg.as_str() {
            "--json" => format = export::OutputFormat::Json,
            "--csv" => format = export::OutputFormat::Csv,
            "--images" | "--extract-images" => export_images = true,
            "--bgm" => export_bgm = true,
            "--maps" => export_maps = true,
            "--loading-images" | "--loadingimage" => export_loading_images = true,
            "--lang" | "--language" => {
                index += 1;
                let value = args.get(index).ok_or("Missing language after --lang.")?;
                language = Some(parse_language(value)?);
            }
            "--hd-images" | "--high-res-images" => {
                export_images = true;
                high_res_images = true;
            }
            "--uld" => {
                export_uld = true;
                if let Some(value) = args.get(index + 1) {
                    if !value.starts_with("--") {
                        index += 1;
                        uld_path_list = Some(value.clone());
                    }
                }
            }
            "--uld-path-list" => {
                export_uld = true;
                index += 1;
                let value = args
                    .get(index)
                    .ok_or("Missing path list after --uld-path-list.")?;
                uld_path_list = Some(value.clone());
            }
            "--format" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or("Missing export format after --format.")?;
                format = parse_output_format(value)?;
            }
            value => {
                if let Some(value) = value.strip_prefix("--format=") {
                    format = parse_output_format(value)?;
                } else if let Some(value) = value.strip_prefix("--lang=") {
                    language = Some(parse_language(value)?);
                } else if let Some(value) = value.strip_prefix("--language=") {
                    language = Some(parse_language(value)?);
                } else {
                    format = parse_output_format(value)?;
                }
            }
        }

        index += 1;
    }

    Ok(Options {
        format,
        export_images,
        high_res_images,
        export_bgm,
        export_maps,
        export_loading_images,
        uld_path_list,
        export_uld,
        language,
    })
}

fn parse_output_format(value: &str) -> Result<export::OutputFormat, Box<dyn Error>> {
    match value.to_ascii_lowercase().as_str() {
        "csv" => Ok(export::OutputFormat::Csv),
        "json" => Ok(export::OutputFormat::Json),
        _ => Err(format!(
            "Invalid option or export format: {value}. Use csv, json, --images, --hd-images, --bgm, or --uld."
        )
        .into()),
    }
}

fn parse_language(value: &str) -> Result<Language, Box<dyn Error>> {
    match value.to_ascii_lowercase().as_str() {
        "en" | "eng" | "english" => Ok(Language::English),
        "de" | "ger" | "german" => Ok(Language::German),
        "fr" | "fre" | "french" => Ok(Language::French),
        "ja" | "jp" | "jpn" | "japanese" => Ok(Language::Japanese),
        "chs" | "zh-cn" | "chinese-simplified" => Ok(Language::ChineseSimplified),
        "ko" | "kr" | "kor" | "korean" => Ok(Language::Korean),
        "tc" | "zh-tw" | "chinese-traditional" => Ok(Language::ChineseTraditional),
        _ => Err(format!("Invalid language: {value}. Use en, de, fr, ja, chs, ko, or tc.").into()),
    }
}
