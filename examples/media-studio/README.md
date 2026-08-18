# Yaiko Media Studio Example

This command-line example demonstrates the feature-gated `persistent-media` integration. It creates a SQLite-backed project repository, stores an asset, advances the optimistic revision, and persists a timeline describing captions and background music.

## Run

```bash
cargo run --manifest-path examples/media-studio/Cargo.toml
```

The example creates `media-studio.db` in the current working directory. The repository is implemented by `MediaEditorRepository` and validates project scope, optimistic revisions, duplicate assets, and capacity limits.
