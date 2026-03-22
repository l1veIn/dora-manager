/// Port Schema — machine-readable data contracts for dora node ports.
///
/// Provides types for the DM Port Schema specification:
/// - `PortSchema` / `ArrowType` — the data model
/// - `parse_schema` — JSON parser with `$ref` resolution
/// - `check_compatibility` — transpile-time type compatibility checker
mod compat;
mod model;
mod parse;

#[cfg(test)]
mod tests;

pub use compat::{check_compatibility, SchemaError};
pub use model::{ArrowType, DateUnit, FloatPrecision, PortSchema, TimeUnit};
pub use parse::parse_schema;
