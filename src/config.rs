use std::{error::Error, fs::File};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub path: String,
    pub raw_sheets: Vec<String>,
    pub translated_sheets: Vec<String>,
}

pub fn read() -> Result<Config, Box<dyn Error>> {
    let file = File::open("config.yml")?;
    let config: Config = serde_yml::from_reader(file)?;

    Ok(config)
}
