use csv::Writer;
use std::error::Error;

use ironworks::excel::{Excel, Field};

use crate::exd_schema;

pub fn sheet(excel: &Excel, sheet_name: &str) -> Result<(), Box<dyn Error>> {
    let field_names = exd_schema::field_names(sheet_name)?;
    let sheet = excel.sheet(sheet_name)?;
    let columns = sheet.columns()?;

    // Sort by offset vs default sort by index
    // columns.sort_by_key(|column| column.offset);

    let path = format!("output/{}.csv", sheet_name);
    let mut writer = Writer::from_path(path)?;

    // Write the field names header
    writer.serialize(&field_names)?;

    for row in sheet.into_iter() {
        let row = &row?;
        let mut data: Vec<String> = Vec::new();

        // TODO: Support row_id.subrow_id
        data.push(row.row_id().to_string());

        // TODO: We need to reference the field by the offset, not index, in order to match EXDSchema
        for i in 0..columns.len() {
            data.push(field_to_string(row.field(i)?));
        }

        writer.serialize(data)?;
    }

    writer.flush()?;

    return Ok(());
}

fn field_to_string(field: Field) -> String {
    return match field {
        // TODO: Figure out formatting for complex strings (e.g. descriptions, tooltips)
        Field::String(value) => value.format().unwrap(),
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
    };
}
