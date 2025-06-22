use std::error::Error;
use std::path::Path;

use ironworks::{
    Ironworks,
    excel::{Excel, Language},
    sqpack::{Install, SqPack},
};

mod exd_schema;
mod export;

fn main() -> Result<(), Box<dyn Error>> {
    let path = Path::new("D:\\SquareEnix\\FINAL FANTASY XIV - A Realm Reborn");
    let ironworks = Ironworks::new().with_resource(SqPack::new(Install::at(path)));
    let language = Language::English;
    let excel = Excel::new(ironworks).with_default_language(language);

    export::sheet(&excel, &language, "Mount")?;

    return Ok(());
}
