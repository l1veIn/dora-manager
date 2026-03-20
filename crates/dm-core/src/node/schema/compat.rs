use std::fmt;

use super::model::*;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Describes why two port schemas are incompatible.
#[derive(Debug, Clone)]
pub enum SchemaError {
    /// Top-level Arrow type family mismatch (e.g. utf8 vs int).
    TypeMismatch {
        output: String,
        input: String,
    },
    /// Integer/float bit width cannot be safely widened.
    UnsafeWidening {
        output: String,
        input: String,
    },
    /// Fixed-size list sizes differ.
    ListSizeMismatch {
        output_size: usize,
        input_size: usize,
    },
    /// Element types of list/fixedsizelist are incompatible.
    IncompatibleItems {
        reason: Box<SchemaError>,
    },
    /// Input struct requires a field not present in output struct.
    MissingStructField {
        field_name: String,
    },
    /// A struct field's schema is incompatible between output and input.
    IncompatibleStructField {
        field_name: String,
        reason: Box<SchemaError>,
    },
    /// Input requires items/properties but output schema lacks them.
    MissingNestedSchema {
        detail: String,
    },
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::TypeMismatch { output, input } => {
                write!(f, "type mismatch: output is '{}', input expects '{}'", output, input)
            }
            SchemaError::UnsafeWidening { output, input } => {
                write!(
                    f,
                    "unsafe widening: output '{}' cannot be safely cast to input '{}'",
                    output, input
                )
            }
            SchemaError::ListSizeMismatch {
                output_size,
                input_size,
            } => {
                write!(
                    f,
                    "fixed-size list size mismatch: output has {}, input expects {}",
                    output_size, input_size
                )
            }
            SchemaError::IncompatibleItems { reason } => {
                write!(f, "incompatible list items: {}", reason)
            }
            SchemaError::MissingStructField { field_name } => {
                write!(
                    f,
                    "output struct is missing required field '{}'",
                    field_name
                )
            }
            SchemaError::IncompatibleStructField { field_name, reason } => {
                write!(
                    f,
                    "struct field '{}' is incompatible: {}",
                    field_name, reason
                )
            }
            SchemaError::MissingNestedSchema { detail } => {
                write!(f, "missing nested schema: {}", detail)
            }
        }
    }
}

impl std::error::Error for SchemaError {}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check whether an output port schema is compatible with an input port schema.
///
/// Compatibility means the output's data can be consumed by the input without
/// data loss or type errors. The check follows subtype semantics:
/// - For structs: output must provide all fields that input requires.
/// - For primitives: safe widening (e.g. int32 → int64) is allowed.
/// - For lists: element types must be compatible.
pub fn check_compatibility(output: &PortSchema, input: &PortSchema) -> Result<(), SchemaError> {
    check_type_compat(&output.arrow_type, &input.arrow_type, output, input)
}

// ---------------------------------------------------------------------------
// Internal compatibility logic
// ---------------------------------------------------------------------------

fn check_type_compat(
    out_type: &ArrowType,
    in_type: &ArrowType,
    out_schema: &PortSchema,
    in_schema: &PortSchema,
) -> Result<(), SchemaError> {
    // Exact match — always OK
    if out_type == in_type {
        // For composite types, recurse into children
        return match out_type {
            ArrowType::List | ArrowType::LargeList | ArrowType::FixedSizeList { .. } => {
                check_items_compat(out_schema, in_schema)
            }
            ArrowType::Struct => check_struct_compat(out_schema, in_schema),
            _ => Ok(()),
        };
    }

    // Safe widening rules
    match (out_type, in_type) {
        // int → int: same signedness, output narrower than input
        (
            ArrowType::Int {
                bit_width: out_bw,
                is_signed: out_s,
            },
            ArrowType::Int {
                bit_width: in_bw,
                is_signed: in_s,
            },
        ) => {
            if out_s != in_s {
                return Err(SchemaError::TypeMismatch {
                    output: out_type.to_string(),
                    input: in_type.to_string(),
                });
            }
            if out_bw <= in_bw {
                Ok(())
            } else {
                Err(SchemaError::UnsafeWidening {
                    output: out_type.to_string(),
                    input: in_type.to_string(),
                })
            }
        }

        // float → float: output precision ≤ input precision
        (
            ArrowType::FloatingPoint {
                precision: out_prec,
            },
            ArrowType::FloatingPoint {
                precision: in_prec,
            },
        ) => {
            if out_prec.bit_width() <= in_prec.bit_width() {
                Ok(())
            } else {
                Err(SchemaError::UnsafeWidening {
                    output: out_type.to_string(),
                    input: in_type.to_string(),
                })
            }
        }

        // utf8 → largeutf8
        (ArrowType::Utf8, ArrowType::LargeUtf8) => Ok(()),

        // binary → largebinary
        (ArrowType::Binary, ArrowType::LargeBinary) => Ok(()),

        // fixedsizelist → list (fixed is a subtype of variable-length)
        (ArrowType::FixedSizeList { .. }, ArrowType::List) => {
            check_items_compat(out_schema, in_schema)
        }

        // fixedsizelist → largelist
        (ArrowType::FixedSizeList { .. }, ArrowType::LargeList) => {
            check_items_compat(out_schema, in_schema)
        }

        // list → largelist
        (ArrowType::List, ArrowType::LargeList) => check_items_compat(out_schema, in_schema),

        // Everything else is a type mismatch
        _ => Err(SchemaError::TypeMismatch {
            output: out_type.to_string(),
            input: in_type.to_string(),
        }),
    }
}

/// Check compatibility of list element schemas.
fn check_items_compat(out_schema: &PortSchema, in_schema: &PortSchema) -> Result<(), SchemaError> {
    match (&out_schema.items, &in_schema.items) {
        (Some(out_items), Some(in_items)) => {
            check_compatibility(out_items, in_items).map_err(|e| SchemaError::IncompatibleItems {
                reason: Box::new(e),
            })
        }
        // If input doesn't declare items, it accepts anything
        (_, None) => Ok(()),
        // If output doesn't declare items but input does, we can't verify
        (None, Some(_)) => Err(SchemaError::MissingNestedSchema {
            detail: "output list has no 'items' schema but input requires one".to_string(),
        }),
    }
}

/// Check struct compatibility using subtype/subset semantics.
///
/// Every field listed in the input's `required` array must exist in the
/// output's `properties`, and their schemas must be compatible.
fn check_struct_compat(out_schema: &PortSchema, in_schema: &PortSchema) -> Result<(), SchemaError> {
    let out_props = out_schema.properties.as_ref();
    let in_props = in_schema.properties.as_ref();

    // If input has no properties constraint, any struct is fine
    let Some(in_props) = in_props else {
        return Ok(());
    };

    let out_props = out_props.ok_or_else(|| SchemaError::MissingNestedSchema {
        detail: "output struct has no 'properties' but input requires specific fields".to_string(),
    })?;

    // Check every required input field
    let required = in_schema.required.as_deref().unwrap_or(&[]);
    for field_name in required {
        let Some(out_field) = out_props.get(field_name) else {
            return Err(SchemaError::MissingStructField {
                field_name: field_name.clone(),
            });
        };
        let Some(in_field) = in_props.get(field_name) else {
            continue; // shouldn't happen if required ⊂ properties, but be safe
        };
        check_compatibility(out_field, in_field).map_err(|e| {
            SchemaError::IncompatibleStructField {
                field_name: field_name.clone(),
                reason: Box::new(e),
            }
        })?;
    }

    // Also check non-required fields that appear in BOTH sides
    for (field_name, in_field) in in_props {
        if required.contains(field_name) {
            continue; // already checked
        }
        if let Some(out_field) = out_props.get(field_name) {
            check_compatibility(out_field, in_field).map_err(|e| {
                SchemaError::IncompatibleStructField {
                    field_name: field_name.clone(),
                    reason: Box::new(e),
                }
            })?;
        }
        // Non-required field missing from output is OK
    }

    Ok(())
}
