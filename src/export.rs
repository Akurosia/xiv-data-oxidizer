//use csv::Writer;
use ironworks::sestring::format::Input;
use std::error::Error;
use std::fs::File;
use std::io::BufWriter;

use ironworks::excel::{Excel, Field, Language};
use ironworks::file::exh::{ColumnDefinition, SheetKind};

use crate::exd_schema::field_names;
use crate::formatter::format_string;
use serde_json::{json, Value};

use std::collections::BTreeMap;

/// Generates a CSV (and optionally JSON) extract for the given sheet, language, and optional suffix
pub fn sheet_with_suffix(excel: &Excel, _language: Language, sheet_name: &str, suffix: Option<String>, write_json: bool) -> Result<(), Box<dyn Error>> {
    let input = Input::new().with_global_parameter(1, String::from("Player Player"));
    let field_names = field_names(sheet_name)?;
    let sheet = excel.sheet(sheet_name)?;
    let has_subrows = sheet.kind()? == SheetKind::Subrows;

    // Sort columns by offset
    let mut columns = sheet.columns()?;
    columns.sort_by_key(|column| column.offset);

    // Generate filenames
    let base_filename = match suffix {
        Some(ref sfx) => format!("output/{}.{}", sheet_name, sfx),
        None => format!("output/{}", sheet_name),
    };

    // Setup CSV writer
    //let csv_path = format!("{}.csv", base_filename);
    //let mut csvwriter = Writer::from_path(&csv_path)?;

    // Write headers to CSV
    //csvwriter.serialize(&field_names)?;

    let mut json_rows: BTreeMap<String, Value> = BTreeMap::new();

    for row in sheet.into_iter() {
        let row = &row?;
        let id = if has_subrows {
            format!("{}.{}", row.row_id(), row.subrow_id())
        } else {
            row.row_id().to_string()
        };

        let mut data: Vec<String> = vec![id.clone()];
        let mut json_object = serde_json::Map::new();

        // Erstes Feld ist immer die ID
        if let crate::exd_schema::FieldName::Simple(ref id_name) = field_names[0] {
            json_object.insert(id_name.clone(), json!(id));
        }

        let mut col_idx = 0;
        for field_name in field_names.iter().skip(1) {
            match field_name {
                crate::exd_schema::FieldName::Simple(name) => {
                    if col_idx >= columns.len() { break; }
                    let column = &columns[col_idx];
                    let specifier = ColumnDefinition {
                        kind: column.kind,
                        offset: column.offset,
                    };
                    let field = row.field(&specifier)?;
                    let string_value = field_to_string(&field, &input);
                    data.push(string_value.clone());
                    let mut out_name = name.clone();
                    if out_name.starts_with("Unknown") {
                        out_name = format!("col_{}", col_idx + 1);
                    }
                    json_object.insert(out_name, json!(string_value));
                    col_idx += 1;
                }
                crate::exd_schema::FieldName::Array(name, keys) => {
                    let mut arr_map = serde_json::Map::new();
                    for key in keys {
                        if col_idx >= columns.len() { break; }
                        let column = &columns[col_idx];
                        let specifier = ColumnDefinition {
                            kind: column.kind,
                            offset: column.offset,
                        };
                        let field = row.field(&specifier)?;
                        let string_value = field_to_string(&field, &input);
                        data.push(string_value.clone());
                        arr_map.insert(key.clone(), json!(string_value));
                        col_idx += 1;
                    }
                    json_object.insert(name.clone(), Value::Object(arr_map));
                }
            }
        }

        //csvwriter.serialize(data)?;

        if write_json {
            if let Some(Value::String(id_str)) = json_object.get("#") {
                let id_str = id_str.clone(); // End the immutable borrow here

                json_object.remove("#"); // Now it's safe to mutably borrow
                json_rows.insert(id_str, Value::Object(json_object));
            }
        }
    }

    //csvwriter.flush()?;

    if write_json {
        let json_path = format!("{}.json", base_filename);
        let file = File::create(json_path)?;
        let jsonwriter = BufWriter::new(file);
        serde_json::to_writer_pretty(jsonwriter, &json_rows)?;
    }

    Ok(())
}

/// Converts a field to string value for export
fn field_to_string(field: &Field, input: &Input) -> String {
    match field {
        Field::String(value) => format_string(value, input),
        Field::Bool(value) => value.to_string(),
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
