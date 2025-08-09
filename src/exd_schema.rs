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
    name: Option<String>, // Name is optional for array fields
    pending_name: Option<String>,

    #[serde(rename = "type", default)]
    kind: FieldKind,

    count: Option<u32>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum FieldKind {
    Scalar,
    Array,
    Icon,
    ModelId,
    Color,
    Link,
}

impl Default for FieldKind {
    fn default() -> Self {
        Self::Scalar
    }
}


/// Enum für Feldnamen, um Arrays als Map darzustellen
#[derive(Debug, Clone)]
pub enum FieldName {
    Simple(String),
    Array(String, Vec<String>), // z.B. ("Data", ["0", "1", ...])
}

/// Retrieve a list of field names from EXDSchema for the given sheet
pub fn field_names(sheet_name: &str) -> Result<Vec<FieldName>, Box<dyn Error>> {
    let path = format!("schemas/{}.yml", sheet_name);
    let file = File::open(path)?;
    let schema: Schema = serde_yml::from_reader(file)?;

    // Prefer the pending field list when available
    let names: Vec<FieldName> = match schema.pending_fields {
        Some(pending) => parse_field_names(&pending),
        None => parse_field_names(&schema.fields),
    };

    Ok(names)
}

fn parse_field_names(fields: &Vec<Field>) -> Vec<FieldName> {
    let mut names: Vec<FieldName> = Vec::new();
    names.push(FieldName::Simple("#".to_string()));

    for field in fields.iter() {
        let name = {
            let n = latest_name(&field);
            if n == "Unknown" {
                format!("col_{}", names.len())
            } else {
                n
            }
        };
        match field.kind {
            FieldKind::Array => {
                let arr_keys = array_keys(field);
                names.push(FieldName::Array(name, arr_keys));
            }
            _ => {
                names.push(FieldName::Simple(name));
            }
        }
    }
    names
}

fn latest_name(field: &Field) -> String {
    if let Some(pending) = &field.pending_name {
        pending.clone()
    } else if let Some(name) = &field.name {
        name.clone()
    } else {
        "Unknown".to_string()
    }
}

/// Liefert die Keys für ein Array-Feld (z.B. ["0", "1", ...])
fn array_keys(field: &Field) -> Vec<String> {
    match &field.count {
        Some(count) => (0..*count).map(|i| i.to_string()).collect(),
        None => vec![],
    }
}
