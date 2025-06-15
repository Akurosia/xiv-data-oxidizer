use serde::Deserialize;
use serde_yml;
use std::error::Error;
use std::fs::File;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Schema {
    fields: Vec<Field>,
    pending_fields: Option<Vec<Field>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Field {
    name: String,
    pending_name: Option<String>,
}

pub fn field_names(sheet_name: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let path = format!("schemas/{}.yml", sheet_name);
    let file = File::open(path)?;
    let schema: Schema = serde_yml::from_reader(file)?;

    // Prefer the pending field list when available
    let names: Vec<String> = match schema.pending_fields {
        Some(fields) => parse_field_names(&fields),
        None => parse_field_names(&schema.fields),
    };

    return Ok(names);
}

fn parse_field_names(fields: &Vec<Field>) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();

    // Add the ID field
    names.push(String::from("#"));

    // TODO: Check the field type and parse array fields properly
    for field in fields.iter() {
        // Prefer the pending field name when available
        let latest_name = match &field.pending_name {
            Some(name) => name,
            None => &field.name,
        };

        names.push(latest_name.clone());
    }

    return names;
}
