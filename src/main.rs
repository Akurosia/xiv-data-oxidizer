use std::error::Error;
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

use ironworks::{
    Ironworks,
    excel::{Excel, Language},
    sqpack::{Install, SqPack},
};

mod config;
mod exd_schema;
mod export;
mod formatter;

const LANGUAGES: [Language; 4] = [
    Language::English,
    Language::German,
    Language::French,
    Language::Japanese,
];
fn is_language_enabled(code: &str, config: &config::Config) -> bool {
    config.languages.is_empty() || config.languages == vec!["*"] || config.languages.contains(&code.to_string())
}


pub fn language_code(language: &Language) -> &str {
    match language {
        Language::English => "en",
        Language::German => "de",
        Language::French => "fr",
        Language::Japanese => "ja",
        Language::Korean => "kr",
        Language::ChineseSimplified => "chs",
        Language::ChineseTraditional => "cht",
        _ => "??",
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = config::read().expect("Could not read config");
    let path = Path::new(&config.path);

    let ironworks = Arc::new(Ironworks::new().with_resource(SqPack::new(Install::at(path))));
    let excel = Excel::new(ironworks.clone()).with_default_language(Language::English);

    // Filter out unwanted sheets
    let mut all_sheets = excel
        .list()?
        .iter()
        .filter(|s| {
            let s = s.as_ref();
            !s.starts_with("custom/")
                && !s.starts_with("quest/")
                && !s.starts_with("cut_scene/")
                && !s.starts_with("dungeon/")
                && !s.starts_with("raid/")
                && !s.starts_with("shop/")
                && !s.starts_with("story/")
                && !s.starts_with("guild_order/")
                && !s.starts_with("content/")
                && !s.starts_with("opening/")
                && !s.starts_with("warp/")
                && !s.starts_with("system/")
                && !s.starts_with("leve/")
                && !s.starts_with("transport/")
        })
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    all_sheets.sort();

    let excel = Arc::new(Mutex::new(excel));

    // Type: (Language, sheet_name, Option<language_code_suffix>)
    let mut tasks: Vec<(Language, String, Option<String>)> = Vec::new();

    {
        let excel_lock = excel.lock().unwrap();

        // Precompute flags
        let raw_all = config.raw_sheets.len() == 1 && config.raw_sheets[0] == "*";
        let trans_all = config.translated_sheets.len() == 1 && config.translated_sheets[0] == "*";

        for sheet in &all_sheets {
            let supported_languages = match excel_lock.sheet(sheet) {
                Ok(sheet) => sheet.languages().unwrap_or_else(|_| vec![]),
                Err(_) => vec![],
            };

            if supported_languages.len() == 1 {
                // Single-language sheet: raw export
                if raw_all || config.raw_sheets.contains(sheet) {
                    tasks.push((supported_languages[0], sheet.clone(), None));
                }
            } else {
                // Multi-language sheet: translated export
                if trans_all || config.translated_sheets.contains(sheet) {
                    for lang in LANGUAGES {
                        let code = language_code(&lang);
                        if is_language_enabled(code, &config) {
                            tasks.push((lang, sheet.clone(), Some(code.to_string())));
                        }
                    }
                }
            }
        }
    }


    let total_tasks = tasks.len();

    // Progress bar (unchanged)
    let pb = ProgressBar::new(total_tasks as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    let pb = Arc::new(pb);

    // Parallel export
    tasks.par_iter().for_each(|(language, sheet_name, suffix)| {
        let excel = excel.clone();
        let pb = pb.clone();

        let result = {
            let mut excel_lock = excel.lock().unwrap();
            excel_lock.set_default_language(*language);
            export::sheet_with_suffix(&*excel_lock, *language, sheet_name, suffix.clone(), true)
        };

        match result {
            Ok(_) => pb.inc(1),
            Err(e) => {
                pb.println(format!(
                    "⚠️ Failed: {} [{}] - {}",
                    sheet_name,
                    language_code(language),
                    e
                ));
                pb.inc(1);
            }
        }
    });


    pb.finish_with_message("✅ All exports complete.");
    Ok(())
}
