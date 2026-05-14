use std::path::Path;
use std::{env, error::Error};

use ironworks::{
    Ironworks,
    excel::Excel,
    sqpack::{Install, SqPack},
};
mod exd_schema;
mod export;
mod formatter;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!(
            "You must provide a game path. For example: cargo run -- \"C:\\Program Files (x86)\\Square Enix\\FINAL FANTASY XIV - A Realm Reborn\" --format csv"
        );
    }

    let path = Path::new(&args[1]);
    let format = parse_output_format(&args[2..])?;

    let ironworks = Ironworks::new().with_resource(SqPack::new(Install::at(path)));
    let languages = export::available_languages(&ironworks);
    let mut excel = Excel::new(ironworks);

    for language in languages {
        excel.set_default_language(language);
        let sheets = excel.list().expect("Could not retrieve sheet list.");

        println!(
            "Exporting {} sheets as {}",
            export::language_code(&language).to_uppercase(),
            format.extension().to_uppercase()
        );

        for sheet in sheets.iter() {
            match export::sheet(&excel, language, &sheet, format) {
                Ok(_) => (),
                // Log failed sheets and continue
                Err(err) => eprintln!("Failed to export {}. {}", sheet, err),
            }
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

fn parse_output_format(args: &[String]) -> Result<export::OutputFormat, Box<dyn Error>> {
    if args.is_empty() {
        return Ok(export::OutputFormat::Csv);
    }

    let value = match args {
        [flag] if flag == "--json" => "json",
        [flag] if flag == "--csv" => "csv",
        [format] => format,
        [flag, format] if flag == "--format" => format,
        [format, flag] if flag == "--format" => format,
        _ => {
            return Err(
                "Invalid export format. Use --format csv, --format json, --csv, or --json.".into(),
            );
        }
    };

    let value = value.strip_prefix("--format=").unwrap_or(value);

    match value.to_ascii_lowercase().as_str() {
        "csv" => Ok(export::OutputFormat::Csv),
        "json" => Ok(export::OutputFormat::Json),
        _ => Err("Invalid export format. Use csv or json.".into()),
    }
}
