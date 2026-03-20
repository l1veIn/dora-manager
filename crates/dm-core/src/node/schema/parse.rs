use std::path::Path;

use anyhow::{bail, Context, Result};

use super::model::*;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a JSON value into a `PortSchema`.
///
/// If the value contains a `$ref`, the referenced file is loaded relative
/// to `base_dir` (typically the node directory).
pub fn parse_schema(value: &serde_json::Value, base_dir: &Path) -> Result<PortSchema> {
    // Handle $ref at the top level
    if let Some(ref_str) = value.get("$ref").and_then(|v| v.as_str()) {
        let resolved = resolve_ref(ref_str, base_dir)?;
        return parse_schema_inner(&resolved);
    }
    parse_schema_inner(value)
}

// ---------------------------------------------------------------------------
// Internal parsing
// ---------------------------------------------------------------------------

fn parse_schema_inner(value: &serde_json::Value) -> Result<PortSchema> {
    let obj = value
        .as_object()
        .context("port schema must be a JSON object")?;

    let arrow_type = obj
        .get("type")
        .context("port schema missing required 'type' field")?;
    let arrow_type = parse_arrow_type(arrow_type)?;

    let items = if let Some(items_val) = obj.get("items") {
        Some(Box::new(parse_schema_inner(items_val)?))
    } else {
        None
    };

    let properties = if let Some(props_val) = obj.get("properties") {
        let props_obj = props_val
            .as_object()
            .context("'properties' must be a JSON object")?;
        let mut map = std::collections::BTreeMap::new();
        for (key, val) in props_obj {
            map.insert(key.clone(), parse_schema_inner(val)?);
        }
        Some(map)
    } else {
        None
    };

    let required = if let Some(req_val) = obj.get("required") {
        let arr = req_val
            .as_array()
            .context("'required' must be an array of strings")?;
        let mut names = Vec::new();
        for v in arr {
            names.push(
                v.as_str()
                    .context("'required' array elements must be strings")?
                    .to_string(),
            );
        }
        Some(names)
    } else {
        None
    };

    Ok(PortSchema {
        id: obj.get("$id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        title: obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        description: obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        arrow_type,
        nullable: obj
            .get("nullable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        items,
        properties,
        required,
        metadata: obj.get("metadata").cloned(),
    })
}

// ---------------------------------------------------------------------------
// Arrow type parsing
// ---------------------------------------------------------------------------

fn parse_arrow_type(value: &serde_json::Value) -> Result<ArrowType> {
    let obj = value
        .as_object()
        .context("'type' must be a JSON object with a 'name' field")?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .context("'type' object missing 'name' field")?;

    match name {
        "null" => Ok(ArrowType::Null),
        "bool" => Ok(ArrowType::Bool),

        "int" => {
            let bit_width = obj
                .get("bitWidth")
                .and_then(|v| v.as_u64())
                .context("int type requires 'bitWidth'")? as u16;
            let is_signed = obj
                .get("isSigned")
                .and_then(|v| v.as_bool())
                .context("int type requires 'isSigned'")?;
            Ok(ArrowType::Int {
                bit_width,
                is_signed,
            })
        }

        "floatingpoint" => {
            let precision_str = obj
                .get("precision")
                .and_then(|v| v.as_str())
                .context("floatingpoint type requires 'precision'")?;
            let precision = match precision_str {
                "HALF" => FloatPrecision::Half,
                "SINGLE" => FloatPrecision::Single,
                "DOUBLE" => FloatPrecision::Double,
                other => bail!("unknown float precision: '{}'", other),
            };
            Ok(ArrowType::FloatingPoint { precision })
        }

        "utf8" => Ok(ArrowType::Utf8),
        "largeutf8" => Ok(ArrowType::LargeUtf8),
        "binary" => Ok(ArrowType::Binary),
        "largebinary" => Ok(ArrowType::LargeBinary),

        "fixedsizebinary" => {
            let byte_width = obj
                .get("byteWidth")
                .and_then(|v| v.as_u64())
                .context("fixedsizebinary type requires 'byteWidth'")?
                as usize;
            Ok(ArrowType::FixedSizeBinary { byte_width })
        }

        "date" => {
            let unit_str = obj
                .get("unit")
                .and_then(|v| v.as_str())
                .context("date type requires 'unit'")?;
            let unit = match unit_str {
                "DAY" => DateUnit::Day,
                "MILLISECOND" => DateUnit::Millisecond,
                other => bail!("unknown date unit: '{}'", other),
            };
            Ok(ArrowType::Date { unit })
        }

        "time" => {
            let unit_str = obj
                .get("unit")
                .and_then(|v| v.as_str())
                .context("time type requires 'unit'")?;
            let unit = parse_time_unit(unit_str)?;
            let bit_width = obj
                .get("bitWidth")
                .and_then(|v| v.as_u64())
                .context("time type requires 'bitWidth'")?
                as u16;
            Ok(ArrowType::Time { unit, bit_width })
        }

        "timestamp" => {
            let unit_str = obj
                .get("unit")
                .and_then(|v| v.as_str())
                .context("timestamp type requires 'unit'")?;
            let unit = parse_time_unit(unit_str)?;
            let timezone = obj
                .get("timezone")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Ok(ArrowType::Timestamp { unit, timezone })
        }

        "duration" => {
            let unit_str = obj
                .get("unit")
                .and_then(|v| v.as_str())
                .context("duration type requires 'unit'")?;
            let unit = parse_time_unit(unit_str)?;
            Ok(ArrowType::Duration { unit })
        }

        "list" => Ok(ArrowType::List),
        "largelist" => Ok(ArrowType::LargeList),

        "fixedsizelist" => {
            let list_size = obj
                .get("listSize")
                .and_then(|v| v.as_u64())
                .context("fixedsizelist type requires 'listSize'")?
                as usize;
            Ok(ArrowType::FixedSizeList { list_size })
        }

        "struct" => Ok(ArrowType::Struct),

        "map" => {
            let keys_sorted = obj
                .get("keysSorted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(ArrowType::Map { keys_sorted })
        }

        other => bail!("unknown Arrow type name: '{}'", other),
    }
}

fn parse_time_unit(s: &str) -> Result<TimeUnit> {
    match s {
        "SECOND" => Ok(TimeUnit::Second),
        "MILLISECOND" => Ok(TimeUnit::Millisecond),
        "MICROSECOND" => Ok(TimeUnit::Microsecond),
        "NANOSECOND" => Ok(TimeUnit::Nanosecond),
        other => bail!("unknown time unit: '{}'", other),
    }
}

// ---------------------------------------------------------------------------
// $ref resolution
// ---------------------------------------------------------------------------

fn resolve_ref(ref_str: &str, base_dir: &Path) -> Result<serde_json::Value> {
    // For now, only relative file paths are supported.
    // Phase 2 will add `dm-schema://` URI resolution.
    let path = base_dir.join(ref_str);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read $ref schema file: {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse $ref schema file: {}", path.display()))?;
    Ok(value)
}
