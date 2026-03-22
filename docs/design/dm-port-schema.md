# DM Port Schema Specification v0.1

A schema format for describing the data contracts of dora-rs node ports. It combines the structural design patterns of [JSON Schema](https://json-schema.org/) with the type system of [Apache Arrow](https://arrow.apache.org/docs/format/Columnar.html#data-types).

## Motivation

Current `dm.json` port definitions only declare an `id`, `direction`, and free-text `description`. When users wire `nodeA/output` → `nodeB/input` in a dataflow YAML, there is **no way to verify whether the data types are compatible** until a runtime error occurs. DM Port Schema solves this by giving every port a machine-readable type contract that the transpiler can validate at compile time.

## Design Principles

1. **Arrow-native types** — The `type` field uses [Arrow's JSON Type representation](https://arrow.apache.org/docs/format/Integration.html#json-test-data-format) directly. No abstraction layer, no mapping table.
2. **JSON Schema ergonomics** — Borrows `$id`, `$ref`, `title`, `description`, `properties`, `required`, `items` from JSON Schema for structure and documentation. Does **not** claim to be a valid JSON Schema document.
3. **Minimal keyword set** — Only keywords that are meaningful for Arrow data are included. No `if/then/else`, no `patternProperties`, no `prefixItems`.
4. **Gradual validation** — Schema validation only triggers when **both** the output port and input port declare a `schema`. Ports without `schema` are silently skipped. Nodes with `"dynamic_ports": true` in `dm.json` may define ports at YAML authoring time that are not pre-declared in `ports`.

## Data Model Premise

All data transmitted between dora-rs nodes is an **Apache Arrow Array**. Even scalar values must be wrapped in an array (e.g. `pa.array(["hello"])` in Python, or a single-element `StringArray` in Rust).

**Port Schema describes the element type of this Arrow Array, not the array itself.** When a schema declares `"type": { "name": "utf8" }`, it means: "this port transmits a `Utf8Array` where each element is a UTF-8 string." The array-level wrapper is implicit and universal — schema authors never need to specify it.

---

## Keywords Reference

### Universal Keywords

Available on **all** schema objects regardless of Arrow type.

| Keyword | Type | Description |
|---|---|---|
| `$id` | `string` | Unique identifier for this schema, enables cross-file referencing. Convention: `"dm-schema://<name>"` |
| `$ref` | `string` | Reference to another schema. Relative path (e.g. `"schema/audio.json"`) or schema ID (e.g. `"dm-schema://audio-pcm"`) |
| `title` | `string` | Short human-readable name |
| `description` | `string` | Detailed human-readable description |
| `type` | `ArrowType` | **Required.** An Arrow JSON Type object (see [Type System](#type-system)) |
| `nullable` | `boolean` | Whether the value can be null. Default: `false` |
| `metadata` | `object` | Free-form key-value pairs for application-specific annotations |

### Structural Keywords

Available only when the Arrow type implies nested structure.

| Keyword | Applicable Arrow Types | Type | Description |
|---|---|---|---|
| `items` | `list`, `largelist`, `fixedsizelist` | `Schema` | Schema of the list element |
| `properties` | `struct` | `{ [name]: Schema }` | Named child fields of the struct |
| `required` | `struct` | `string[]` | Which `properties` keys are mandatory |

### Constraint Keywords (optional, documentation-only)

These do **not** affect transpile-time type compatibility checks. They serve as machine-readable documentation for node developers.

| Keyword | Applicable Arrow Types | Type | Description |
|---|---|---|---|
| `minimum` | `int`, `floatingpoint`, `decimal` | `number` | Minimum value hint |
| `maximum` | `int`, `floatingpoint`, `decimal` | `number` | Maximum value hint |
| `enum` | `utf8`, `largeutf8`, `int` | `array` | Enumeration of allowed values |
| `default` | any | any | Default value hint |

---

## Type System

The `type` field is an Arrow JSON Type object as defined in the [Arrow Integration Testing specification](https://arrow.apache.org/docs/format/Integration.html#json-test-data-format). The following types are supported:

### Primitive Types

```jsonc
// Null
{ "name": "null" }

// Boolean
{ "name": "bool" }

// Signed integers
{ "name": "int", "bitWidth": 8,  "isSigned": true }   // int8
{ "name": "int", "bitWidth": 16, "isSigned": true }   // int16
{ "name": "int", "bitWidth": 32, "isSigned": true }   // int32
{ "name": "int", "bitWidth": 64, "isSigned": true }   // int64

// Unsigned integers
{ "name": "int", "bitWidth": 8,  "isSigned": false }  // uint8
{ "name": "int", "bitWidth": 16, "isSigned": false }  // uint16
{ "name": "int", "bitWidth": 32, "isSigned": false }  // uint32
{ "name": "int", "bitWidth": 64, "isSigned": false }  // uint64

// Floating point
{ "name": "floatingpoint", "precision": "HALF" }       // float16
{ "name": "floatingpoint", "precision": "SINGLE" }     // float32
{ "name": "floatingpoint", "precision": "DOUBLE" }     // float64
```

### Binary & String Types

```jsonc
{ "name": "binary" }           // Variable-length binary (32-bit offsets)
{ "name": "largebinary" }      // Variable-length binary (64-bit offsets)
{ "name": "utf8" }             // UTF-8 string (32-bit offsets)
{ "name": "largeutf8" }        // UTF-8 string (64-bit offsets)
{ "name": "fixedsizebinary", "byteWidth": 16 }  // Fixed-size binary
```

### Temporal Types

```jsonc
{ "name": "date", "unit": "DAY" }
{ "name": "date", "unit": "MILLISECOND" }
{ "name": "timestamp", "unit": "MICROSECOND", "timezone": "UTC" }
{ "name": "time", "unit": "NANOSECOND", "bitWidth": 64 }
{ "name": "duration", "unit": "MILLISECOND" }
```

### Nested Types

```jsonc
// Variable-length list — use `items` to define element schema
{ "name": "list" }

// Fixed-size list — use `items` to define element schema
{ "name": "fixedsizelist", "listSize": 1600 }

// Struct — use `properties` and `required` to define fields
{ "name": "struct" }

// Map
{ "name": "map", "keysSorted": false }
```

---

## Usage in dm.json

Ports declare their schema via the `schema` key — either inline or as a `$ref`:

```jsonc
{
  "ports": [
    {
      "id": "audio",
      "direction": "output",
      "description": "Continuous PCM audio stream",
      "schema": { "$ref": "schema/audio-pcm.json" }
    },
    {
      "id": "tick",
      "direction": "input",
      "description": "Heartbeat timer",
      "schema": {
        "type": { "name": "null" },
        "description": "Any event, payload is ignored"
      }
    },
    {
      "id": "device_id",
      "direction": "input",
      "description": "Select microphone by device ID",
      "schema": {
        "type": { "name": "utf8" }
      }
    }
  ]
}
```

### Schema File Location

Node-level schemas live in a `schema/` subdirectory within the node directory:

```
nodes/
  dm-microphone/
    dm.json
    schema/
      audio-pcm.json
      device-list.json
    dm_microphone/
      main.py
```

---

## Complete Examples

### Example 1: PCM Audio Stream

`nodes/dm-microphone/schema/audio-pcm.json`:

```json
{
  "$id": "dm-schema://audio-pcm",
  "title": "PCM Audio Chunk",
  "description": "Float32 PCM audio samples, single channel",
  "type": { "name": "fixedsizelist", "listSize": 1600 },
  "nullable": false,
  "items": {
    "type": { "name": "floatingpoint", "precision": "SINGLE" },
    "minimum": -1.0,
    "maximum": 1.0
  }
}
```

### Example 2: RGB Image Frame

```json
{
  "$id": "dm-schema://image-rgb",
  "title": "RGB Image",
  "description": "HxWx3 interleaved RGB image as flat uint8 array",
  "type": { "name": "list" },
  "nullable": false,
  "items": {
    "type": { "name": "int", "bitWidth": 8, "isSigned": false },
    "minimum": 0,
    "maximum": 255
  }
}
```

### Example 3: Object Detection Results

`schema/detections.json`:

```json
{
  "$id": "dm-schema://detections",
  "title": "Object Detection Results",
  "description": "List of detected objects with bounding boxes",
  "type": { "name": "list" },
  "nullable": false,
  "items": {
    "type": { "name": "struct" },
    "required": ["bbox", "label", "confidence"],
    "properties": {
      "bbox": {
        "description": "Bounding box [x1, y1, x2, y2] in pixels",
        "type": { "name": "fixedsizelist", "listSize": 4 },
        "items": {
          "type": { "name": "floatingpoint", "precision": "SINGLE" }
        }
      },
      "label": {
        "description": "Class label",
        "type": { "name": "utf8" }
      },
      "confidence": {
        "description": "Detection confidence",
        "type": { "name": "floatingpoint", "precision": "SINGLE" },
        "minimum": 0,
        "maximum": 1
      }
    }
  }
}
```

### Example 4: Control Signal (simple string command)

```json
{
  "$id": "dm-schema://control-signal",
  "title": "Control Signal",
  "description": "Simple text command for flow control",
  "type": { "name": "utf8" },
  "enum": ["flush", "reset", "stop"]
}
```

### Example 5: Schema Reuse via $ref

A downstream ASR node that consumes the same audio format:

```jsonc
// nodes/dm-asr/dm.json
{
  "ports": [
    {
      "id": "audio_in",
      "direction": "input",
      "description": "Audio input for speech recognition",
      "schema": { "$ref": "dm-schema://audio-pcm" }
    },
    {
      "id": "transcript",
      "direction": "output",
      "description": "Recognized text",
      "schema": {
        "type": { "name": "utf8" }
      }
    }
  ]
}
```

---

## Transpile-Time Validation Rules

When the transpiler encounters a connection `nodeA/output → nodeB/input`:

1. **Load both schemas** — Resolve `$ref` if present, load the schema objects.
2. **Compare Arrow types** — The output's `type` must be **compatible** with the input's `type`.
3. **Report result** — Exact match = pass ✅, incompatible = error ❌.

### Type Compatibility Matrix

| Output Type | Input Type | Result |
|---|---|---|
| Exact same type | — | ✅ Pass |
| `int(32, signed)` | `int(64, signed)` | ✅ Widening (safe) |
| `floatingpoint(SINGLE)` | `floatingpoint(DOUBLE)` | ✅ Widening (safe) |
| `fixedsizelist(N)` | `list` | ✅ Fixed→Variable (safe) |
| `utf8` | `largeutf8` | ✅ Widening (safe) |
| `int(32, signed)` | `floatingpoint(SINGLE)` | ❌ Cross-domain mismatch |
| `utf8` | `int(32, signed)` | ❌ Type mismatch |
| `fixedsizelist(100)` | `fixedsizelist(200)` | ❌ Size mismatch |
| `struct{a,b}` | `struct{a,b,c}` | ❌ Missing field `c` |

> [!NOTE]
> Constraint keywords (`minimum`, `maximum`, `enum`) are **not** checked during transpilation. They are documentation-only hints.

---

## Future: Schema as First-Class Citizen

When the number of shared schemas grows, they can be promoted to a managed resource with a discovery mechanism mirroring nodes:

```
Phase 1 (current):  schema/ lives inside each node directory
Phase 2 (future):   ~/.dm/schemas/         (user-level, highest priority)
                    repository schemas/     (workspace-level)
                    DM_SCHEMA_DIRS env var  (additional directories)
```

Cross-node `$ref` by `$id` (e.g. `"$ref": "dm-schema://audio-pcm"`) will be resolved through this discovery chain.
