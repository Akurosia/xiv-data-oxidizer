use csv::Writer;
use std::error::Error;

use ironworks::excel::{Excel, Field, Language};
use ironworks::file::exh::{ColumnDefinition, SheetKind};

use crate::exd_schema;

pub fn sheet(excel: &Excel, language: &Language, sheet_name: &str) -> Result<(), Box<dyn Error>> {
    // Retrieve the field names based on EXDSchema
    let field_names = exd_schema::field_names(sheet_name)?;

    // Fetch the sheet data
    let sheet = excel.sheet(sheet_name)?;
    let has_subrows = sheet.kind()? == SheetKind::Subrows;

    // Sort by offset to align with EXDSchema column order
    let mut columns = sheet.columns()?;
    columns.sort_by_key(|column| column.offset);

    // Set up the output file
    let language_code = language_code(language);
    let path = format!("output/{}.{}.csv", sheet_name, language_code);
    let mut writer = Writer::from_path(path)?;

    // Write the field names header
    writer.serialize(&field_names)?;

    // Write the file data
    for row in sheet.into_iter() {
        let row = &row?;
        let mut data: Vec<String> = Vec::new();

        let id = match has_subrows {
            true => format!("{}.{}", row.row_id(), row.subrow_id()),
            false => row.row_id().to_string(),
        };

        data.push(id);

        for column in columns.iter() {
            let specifier = ColumnDefinition {
                kind: column.kind,
                offset: column.offset,
            };
            let field = row.field(&specifier)?;

            data.push(field_to_string(&field));
        }

        writer.serialize(data)?;
    }

    writer.flush()?;

    return Ok(());
}

fn language_code(language: &Language) -> &str {
    return match language {
        Language::English => "en",
        Language::German => "de",
        Language::French => "fr",
        Language::Japanese => "ja",
        Language::Korean => "kr",
        Language::ChineseSimplified => "chs",
        Language::ChineseTraditional => "cht",
        _ => "??",
    };
}

fn field_to_string(field: &Field) -> String {
    return match field {
        // TODO: Figure out formatting for complex strings (e.g. descriptions, tooltips)
        Field::String(value) => value.format().unwrap(),
        Field::Bool(value) => {
            if *value {
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
    };
}
