use std::error::Error;
use std::path::Path;

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

fn main() -> Result<(), Box<dyn Error>> {
    let config = config::read().expect("Could not read config");
    let path = Path::new(&config.path);

    let ironworks = Ironworks::new().with_resource(SqPack::new(Install::at(path)));
    let language = Language::English;
    let mut excel = Excel::new(ironworks).with_default_language(language);

    for sheet in config.raw_sheets {
        export::sheet(&excel, language, &sheet)?;
    }

    let translated_sheets = config.translated_sheets;

    for language in LANGUAGES {
        excel.set_default_language(language);

        for sheet in &translated_sheets {
            export::sheet(&excel, language, &sheet)?;
        }
    }

    // Quick debugging for schema updates

    // for language in LANGUAGES {
    //     excel.set_default_language(language);
    //     export::sheet(&excel, language, &String::from("Mount"))?;
    // }

    // export::sheet(&excel, &language, "Mount")?;

    Ok(())
}
