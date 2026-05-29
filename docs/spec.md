# coven-scout MCP Tool Spec

JSON Schema definitions for all tool inputs.

## Tool: read

```json
{
  "type": "object",
  "properties": {
    "operation": { "type": "string", "enum": ["content", "metadata", "diff"] },
    "sources": { "type": "array", "items": { "type": "string" } },
    "format": { "type": "string", "enum": ["text", "base64", "markdown", "checksum"] },
    "checksum_algorithm": { "type": "string", "enum": ["sha256", "sha512", "md5"] },
    "offset": { "type": "integer" },
    "length": { "type": "integer" }
  },
  "required": ["operation", "sources"]
}
```

## Tool: write

```json
{
  "type": "object",
  "properties": {
    "operation": { "type": "string", "enum": ["put","mkdir","copy","move","delete","touch","archive","unarchive"] },
    "entries": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "path": { "type": "string" },
          "content": { "type": "string" },
          "write_mode": { "type": "string", "enum": ["overwrite","append","error_if_exists"] },
          "input_encoding": { "type": "string", "enum": ["text","base64"] },
          "recursive": { "type": "boolean" },
          "source_path": { "type": "string" },
          "destination_path": { "type": "string" }
        }
      }
    },
    "source_paths": { "type": "array", "items": { "type": "string" } },
    "archive_path": { "type": "string" },
    "destination_path": { "type": "string" },
    "format": { "type": "string", "enum": ["zip","tar.gz","tgz"] }
  },
  "required": ["operation"]
}
```

## Tool: list

```json
{
  "type": "object",
  "properties": {
    "operation": { "type": "string", "enum": ["entries","system_info"] },
    "path": { "type": "string" },
    "recursive_depth": { "type": "integer" },
    "calculate_recursive_size": { "type": "boolean" },
    "info_type": { "type": "string", "enum": ["server_capabilities","filesystem_stats"] }
  },
  "required": ["operation"]
}
```

## Tool: find

```json
{
  "type": "object",
  "properties": {
    "operation": { "type": "string", "enum": ["search"] },
    "path": { "type": "string" },
    "recursive": { "type": "boolean" },
    "name_pattern": { "type": "string" },
    "case_sensitive": { "type": "boolean" },
    "content_pattern": { "type": "string" },
    "content_is_regex": { "type": "boolean" },
    "content_case_sensitive": { "type": "boolean" },
    "file_extensions": { "type": "array", "items": { "type": "string" } },
    "size_min": { "type": "integer" },
    "size_max": { "type": "integer" },
    "modified_after": { "type": "string", "format": "date-time" },
    "modified_before": { "type": "string", "format": "date-time" },
    "entry_type": { "type": "string", "enum": ["file","directory","any"] },
    "mime_type": { "type": "string" },
    "max_results": { "type": "integer" }
  },
  "required": ["operation", "path"]
}
```

## Tool: test

```json
{
  "type": "object",
  "properties": {
    "operation": { "type": "string", "enum": ["echo","generate_error"] },
    "params_to_echo": {},
    "error_code": { "type": "string" }
  },
  "required": ["operation"]
}
```
