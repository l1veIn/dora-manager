#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::node::schema::{check_compatibility, parse_schema, ArrowType, FloatPrecision};

    fn parse(value: serde_json::Value) -> super::super::model::PortSchema {
        parse_schema(&value, std::path::Path::new(".")).unwrap()
    }

    // -----------------------------------------------------------------------
    // Parse tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_null() {
        let s = parse(json!({ "type": { "name": "null" } }));
        assert_eq!(s.arrow_type, ArrowType::Null);
    }

    #[test]
    fn parse_bool() {
        let s = parse(json!({ "type": { "name": "bool" } }));
        assert_eq!(s.arrow_type, ArrowType::Bool);
    }

    #[test]
    fn parse_int32() {
        let s = parse(json!({ "type": { "name": "int", "bitWidth": 32, "isSigned": true } }));
        assert_eq!(
            s.arrow_type,
            ArrowType::Int {
                bit_width: 32,
                is_signed: true,
            }
        );
    }

    #[test]
    fn parse_uint8() {
        let s = parse(json!({ "type": { "name": "int", "bitWidth": 8, "isSigned": false } }));
        assert_eq!(
            s.arrow_type,
            ArrowType::Int {
                bit_width: 8,
                is_signed: false,
            }
        );
    }

    #[test]
    fn parse_float32() {
        let s = parse(json!({ "type": { "name": "floatingpoint", "precision": "SINGLE" } }));
        assert_eq!(
            s.arrow_type,
            ArrowType::FloatingPoint {
                precision: FloatPrecision::Single,
            }
        );
    }

    #[test]
    fn parse_utf8() {
        let s = parse(json!({ "type": { "name": "utf8" } }));
        assert_eq!(s.arrow_type, ArrowType::Utf8);
    }

    #[test]
    fn parse_binary() {
        let s = parse(json!({ "type": { "name": "binary" } }));
        assert_eq!(s.arrow_type, ArrowType::Binary);
    }

    #[test]
    fn parse_fixed_size_list() {
        let s = parse(json!({
            "type": { "name": "fixedsizelist", "listSize": 1600 },
            "items": { "type": { "name": "floatingpoint", "precision": "SINGLE" } }
        }));
        assert_eq!(s.arrow_type, ArrowType::FixedSizeList { list_size: 1600 });
        assert!(s.items.is_some());
        assert_eq!(
            s.items.unwrap().arrow_type,
            ArrowType::FloatingPoint {
                precision: FloatPrecision::Single,
            }
        );
    }

    #[test]
    fn parse_struct_with_properties() {
        let s = parse(json!({
            "type": { "name": "struct" },
            "required": ["label", "confidence"],
            "properties": {
                "label": { "type": { "name": "utf8" } },
                "confidence": {
                    "type": { "name": "floatingpoint", "precision": "SINGLE" }
                }
            }
        }));
        assert_eq!(s.arrow_type, ArrowType::Struct);
        assert_eq!(s.required.as_ref().unwrap().len(), 2);
        let props = s.properties.as_ref().unwrap();
        assert_eq!(props.len(), 2);
        assert_eq!(props["label"].arrow_type, ArrowType::Utf8);
    }

    #[test]
    fn parse_timestamp() {
        let s = parse(json!({
            "type": { "name": "timestamp", "unit": "MICROSECOND", "timezone": "UTC" }
        }));
        match &s.arrow_type {
            ArrowType::Timestamp { unit, timezone } => {
                assert_eq!(*unit, super::super::model::TimeUnit::Microsecond);
                assert_eq!(timezone.as_deref(), Some("UTC"));
            }
            other => panic!("expected Timestamp, got {:?}", other),
        }
    }

    #[test]
    fn parse_with_metadata() {
        let s = parse(json!({
            "$id": "dm-schema://test",
            "title": "Test Schema",
            "description": "A test",
            "type": { "name": "utf8" },
            "nullable": true
        }));
        assert_eq!(s.id.as_deref(), Some("dm-schema://test"));
        assert_eq!(s.title.as_deref(), Some("Test Schema"));
        assert!(s.nullable);
    }

    #[test]
    fn parse_missing_type_fails() {
        let result = parse_schema(
            &json!({ "description": "no type" }),
            std::path::Path::new("."),
        );
        assert!(result.is_err());
    }

    #[test]
    fn parse_unknown_type_fails() {
        let result = parse_schema(
            &json!({ "type": { "name": "unknown_type" } }),
            std::path::Path::new("."),
        );
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Compatibility tests
    // -----------------------------------------------------------------------

    #[test]
    fn compat_exact_match() {
        let out = parse(json!({ "type": { "name": "utf8" } }));
        let inp = parse(json!({ "type": { "name": "utf8" } }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }

    #[test]
    fn compat_int_widening_ok() {
        let out = parse(json!({ "type": { "name": "int", "bitWidth": 32, "isSigned": true } }));
        let inp = parse(json!({ "type": { "name": "int", "bitWidth": 64, "isSigned": true } }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }

    #[test]
    fn compat_int_narrowing_fails() {
        let out = parse(json!({ "type": { "name": "int", "bitWidth": 64, "isSigned": true } }));
        let inp = parse(json!({ "type": { "name": "int", "bitWidth": 32, "isSigned": true } }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_int_sign_mismatch_fails() {
        let out = parse(json!({ "type": { "name": "int", "bitWidth": 32, "isSigned": true } }));
        let inp = parse(json!({ "type": { "name": "int", "bitWidth": 32, "isSigned": false } }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_float_widening_ok() {
        let out = parse(json!({ "type": { "name": "floatingpoint", "precision": "SINGLE" } }));
        let inp = parse(json!({ "type": { "name": "floatingpoint", "precision": "DOUBLE" } }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }

    #[test]
    fn compat_float_narrowing_fails() {
        let out = parse(json!({ "type": { "name": "floatingpoint", "precision": "DOUBLE" } }));
        let inp = parse(json!({ "type": { "name": "floatingpoint", "precision": "SINGLE" } }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_utf8_to_largeutf8_ok() {
        let out = parse(json!({ "type": { "name": "utf8" } }));
        let inp = parse(json!({ "type": { "name": "largeutf8" } }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }

    #[test]
    fn compat_cross_domain_fails() {
        let out = parse(json!({ "type": { "name": "utf8" } }));
        let inp = parse(json!({ "type": { "name": "int", "bitWidth": 32, "isSigned": true } }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_fixedsizelist_to_list_ok() {
        let out = parse(json!({
            "type": { "name": "fixedsizelist", "listSize": 100 },
            "items": { "type": { "name": "floatingpoint", "precision": "SINGLE" } }
        }));
        let inp = parse(json!({
            "type": { "name": "list" },
            "items": { "type": { "name": "floatingpoint", "precision": "SINGLE" } }
        }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }

    #[test]
    fn compat_fixedsizelist_size_mismatch_fails() {
        let out = parse(json!({
            "type": { "name": "fixedsizelist", "listSize": 100 },
            "items": { "type": { "name": "floatingpoint", "precision": "SINGLE" } }
        }));
        let inp = parse(json!({
            "type": { "name": "fixedsizelist", "listSize": 200 },
            "items": { "type": { "name": "floatingpoint", "precision": "SINGLE" } }
        }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_list_incompatible_items_fails() {
        let out = parse(json!({
            "type": { "name": "list" },
            "items": { "type": { "name": "utf8" } }
        }));
        let inp = parse(json!({
            "type": { "name": "list" },
            "items": { "type": { "name": "int", "bitWidth": 32, "isSigned": true } }
        }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_struct_superset_ok() {
        let out = parse(json!({
            "type": { "name": "struct" },
            "properties": {
                "a": { "type": { "name": "utf8" } },
                "b": { "type": { "name": "int", "bitWidth": 32, "isSigned": true } },
                "c": { "type": { "name": "floatingpoint", "precision": "SINGLE" } }
            }
        }));
        // Input only requires a and b — output has all of them + extra field c
        let inp = parse(json!({
            "type": { "name": "struct" },
            "required": ["a", "b"],
            "properties": {
                "a": { "type": { "name": "utf8" } },
                "b": { "type": { "name": "int", "bitWidth": 32, "isSigned": true } }
            }
        }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }

    #[test]
    fn compat_struct_missing_required_field_fails() {
        let out = parse(json!({
            "type": { "name": "struct" },
            "properties": {
                "a": { "type": { "name": "utf8" } }
            }
        }));
        let inp = parse(json!({
            "type": { "name": "struct" },
            "required": ["a", "b"],
            "properties": {
                "a": { "type": { "name": "utf8" } },
                "b": { "type": { "name": "int", "bitWidth": 32, "isSigned": true } }
            }
        }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_struct_field_type_mismatch_fails() {
        let out = parse(json!({
            "type": { "name": "struct" },
            "properties": {
                "a": { "type": { "name": "utf8" } },
                "b": { "type": { "name": "utf8" } }
            }
        }));
        let inp = parse(json!({
            "type": { "name": "struct" },
            "required": ["a", "b"],
            "properties": {
                "a": { "type": { "name": "utf8" } },
                "b": { "type": { "name": "int", "bitWidth": 32, "isSigned": true } }
            }
        }));
        assert!(check_compatibility(&out, &inp).is_err());
    }

    #[test]
    fn compat_null_exact_match() {
        let out = parse(json!({ "type": { "name": "null" } }));
        let inp = parse(json!({ "type": { "name": "null" } }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }

    #[test]
    fn compat_input_no_items_accepts_any_list() {
        let out = parse(json!({
            "type": { "name": "list" },
            "items": { "type": { "name": "utf8" } }
        }));
        let inp = parse(json!({
            "type": { "name": "list" }
        }));
        assert!(check_compatibility(&out, &inp).is_ok());
    }
}
