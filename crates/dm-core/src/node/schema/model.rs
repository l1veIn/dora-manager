use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Arrow Type enums
// ---------------------------------------------------------------------------

/// Floating-point precision levels as defined by Arrow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum FloatPrecision {
    Half,
    Single,
    Double,
}

impl FloatPrecision {
    /// Bit width of this precision level.
    pub fn bit_width(&self) -> u16 {
        match self {
            FloatPrecision::Half => 16,
            FloatPrecision::Single => 32,
            FloatPrecision::Double => 64,
        }
    }
}

impl fmt::Display for FloatPrecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FloatPrecision::Half => write!(f, "float16"),
            FloatPrecision::Single => write!(f, "float32"),
            FloatPrecision::Double => write!(f, "float64"),
        }
    }
}

/// Date unit for Arrow Date types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DateUnit {
    Day,
    Millisecond,
}

/// Time unit for Arrow Timestamp/Time/Duration types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeUnit {
    Second,
    Millisecond,
    Microsecond,
    Nanosecond,
}

// ---------------------------------------------------------------------------
// ArrowType — the `type` field in DM Port Schema
// ---------------------------------------------------------------------------

/// An Arrow data type expressed as a JSON Type object.
///
/// This mirrors the Arrow Integration Testing JSON format:
/// <https://arrow.apache.org/docs/format/Integration.html#json-test-data-format>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrowType {
    Null,
    Bool,
    Int {
        bit_width: u16,
        is_signed: bool,
    },
    FloatingPoint {
        precision: FloatPrecision,
    },
    Utf8,
    LargeUtf8,
    Binary,
    LargeBinary,
    FixedSizeBinary {
        byte_width: usize,
    },
    Date {
        unit: DateUnit,
    },
    Time {
        unit: TimeUnit,
        bit_width: u16,
    },
    Timestamp {
        unit: TimeUnit,
        timezone: Option<String>,
    },
    Duration {
        unit: TimeUnit,
    },
    List,
    LargeList,
    FixedSizeList {
        list_size: usize,
    },
    Struct,
    Map {
        keys_sorted: bool,
    },
}

impl fmt::Display for ArrowType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrowType::Null => write!(f, "null"),
            ArrowType::Bool => write!(f, "bool"),
            ArrowType::Int {
                bit_width,
                is_signed,
            } => {
                let prefix = if *is_signed { "int" } else { "uint" };
                write!(f, "{}{}", prefix, bit_width)
            }
            ArrowType::FloatingPoint { precision } => write!(f, "{}", precision),
            ArrowType::Utf8 => write!(f, "utf8"),
            ArrowType::LargeUtf8 => write!(f, "large_utf8"),
            ArrowType::Binary => write!(f, "binary"),
            ArrowType::LargeBinary => write!(f, "large_binary"),
            ArrowType::FixedSizeBinary { byte_width } => {
                write!(f, "fixed_size_binary({})", byte_width)
            }
            ArrowType::Date { unit } => write!(f, "date({:?})", unit),
            ArrowType::Time { unit, bit_width } => {
                write!(f, "time({:?}, {})", unit, bit_width)
            }
            ArrowType::Timestamp { unit, timezone } => {
                if let Some(tz) = timezone {
                    write!(f, "timestamp({:?}, {})", unit, tz)
                } else {
                    write!(f, "timestamp({:?})", unit)
                }
            }
            ArrowType::Duration { unit } => write!(f, "duration({:?})", unit),
            ArrowType::List => write!(f, "list"),
            ArrowType::LargeList => write!(f, "large_list"),
            ArrowType::FixedSizeList { list_size } => {
                write!(f, "fixed_size_list({})", list_size)
            }
            ArrowType::Struct => write!(f, "struct"),
            ArrowType::Map { keys_sorted } => {
                write!(f, "map(keys_sorted={})", keys_sorted)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PortSchema — the top-level schema object
// ---------------------------------------------------------------------------

/// A parsed DM Port Schema.
///
/// Represents the data contract for a single node port.
/// The `arrow_type` field is always present (required by spec).
/// Structural fields (`items`, `properties`, `required`) are only
/// meaningful for the corresponding Arrow nested types.
#[derive(Debug, Clone)]
pub struct PortSchema {
    /// `$id` — unique schema identifier (e.g. `"dm-schema://audio-pcm"`).
    pub id: Option<String>,
    /// `title` — short human-readable name.
    pub title: Option<String>,
    /// `description` — detailed human-readable description.
    pub description: Option<String>,
    /// `type` — the Arrow data type (required).
    pub arrow_type: ArrowType,
    /// `nullable` — whether the value can be null.
    pub nullable: bool,
    /// `items` — element schema for list types.
    pub items: Option<Box<PortSchema>>,
    /// `properties` — named child fields for struct types.
    pub properties: Option<BTreeMap<String, PortSchema>>,
    /// `required` — mandatory property names for struct types.
    pub required: Option<Vec<String>>,
    /// `metadata` — free-form annotations.
    pub metadata: Option<serde_json::Value>,
}
