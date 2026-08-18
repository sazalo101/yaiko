# Yaiko Built-in Catalog Example

This example is a small HTTP application that demonstrates how Yaiko combines routing, health checks, safe document metadata, and structured JSON responses.

## Run

```bash
cargo run --manifest-path examples/catalog/Cargo.toml
```

Open `http://127.0.0.1:3010/` for the metadata-rendered page, `http://127.0.0.1:3010/api/catalog` for the JSON module catalog, and `/health` for the health endpoint.
