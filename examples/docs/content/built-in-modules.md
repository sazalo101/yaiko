# Yaiko Built-in Modules

Yaiko exposes a growing set of production-oriented built-ins from `yaiko_core`. The modules are intentionally small, typed, feature-friendly facades: they validate inputs, keep deterministic ordering where useful, and return structured errors instead of hiding policy decisions.

## Current Catalog

| Area | Representative modules | What they provide |
|---|---|---|
| Runtime and routing | `network_endpoint`, `fs_router`, `router`, `worker` | Validated endpoints, filesystem-style routes, parameter matching, bounded task execution, retries, and cancellation. |
| Frontend primitives | `image`, `font`, `form`, `link`, `head`, `metadata`, `script`, `style`, `icon`, `static_asset` | Safe HTML metadata, responsive images, font sources, forms, navigation hints, scripts, scoped CSS, icons, and cache-aware asset policy. |
| Data layer | `query_client`, `serialize`, `transaction`, `pool`, `data_transfer` | Bounded query caching, deterministic JSON rendering, transaction state helpers, pool policy, and JSON/CSV transfer. |
| Security | `rbac`, `auth`, `cors`, `csp`, `rate_limit`, `media_access` | Role inheritance, wildcard permissions, authentication, origin controls, content-security policy, quotas, and signed media access. |
| API and realtime | `api_facade`, `rpc`, `proxy`, `event_bus`, `pubsub`, `webhook`, `openapi` | Typed route declarations, RPC envelopes, safe upstream policy, ordered events, bounded channels, signed webhooks, and API descriptions. |
| Developer experience | `watch`, `hmr`, `typegen`, `test_facade`, `lint_policy`, `format_policy` | File-watch policy, HMR events, Rust/TypeScript declarations, structured test reports, lint diagnostics, and source normalization. |
| Observability and deployment | `log_facade`, `health_facade`, `metrics`, `tracing_context`, `deploy` | Redacted structured logs, readiness aggregation, metrics/tracing, and validated deployment plans. |
| Media editor | `media_processing`, `media_timeline`, `media_annotations`, `media_review`, `media_asset_versioning`, `media_project_templates`, `media_export_presets`, `media_editor_repository` | Captions, music, timelines, annotations, reviews, immutable versions, reusable projects, safe export profiles, and optional SQLite persistence. |

## Example: Frontend Metadata and Safe Links

```rust
use yaiko_core::{Head, Link, Metadata, NavigationMode, Prefetch};

let head = Head::new()
    .title("Yaiko Blog")
    .meta("description", "A Rust-powered publishing site")?;

let link = Link::new("/posts/hello", "Read the post")?
    .navigation(NavigationMode::Client)
    .prefetch(Prefetch::Intent);

let html = format!("{}{}", head.render(), link.render());
```

The builders reject unsafe sources and attributes before they reach a template. This makes them suitable for server-rendered pages as well as browser-enhanced applications.

## Example: Query Cache and Deterministic Serialization

```rust
use serde_json::json;
use yaiko_core::{QueryClient, Serializer};

let mut cache = QueryClient::new(128);
cache.set("posts:recent", br#"[{\"id\":1}]"#.to_vec(), 100, 30)?;

let serializer = Serializer::new(64 * 1024);
let body = serializer.render(&json!({"ok": true, "count": 1}))?;
```

Both modules enforce bounds. Query entries become stale after their TTL, while serialized output is rejected when it exceeds the configured maximum.

## Example: Media Editor Primitives

```rust
use yaiko_core::{MediaExportPresetStore, MediaReviewStore};

// Use the media stores alongside the existing timeline and FFmpeg specifications.
// Enable `persistent-media` for the SQLx-backed project repository.
```

Media modules provide tested domain contracts. Enable the `persistent-media` feature for `MediaEditorRepository`, which persists project scope, optimistic revisions, ordered assets, and timelines in SQLite. The remaining integration layer is HTTP/WebSocket handlers, controlled workers, and a browser editor.

## Verification Workflow

Every newly added built-in is checked with focused tests, `cargo fmt`, strict Clippy, `git diff --check`, the SQLite/Postgres/metrics/dev feature matrix, CLI tests, and the blog/chat/auth example builds. The repository’s current verified deployment history is recorded in the main README and gap analysis.

## Related Guides

- [Getting Started](getting-started.md)
- [Frontend](frontend.md)
- [Routing](routing.md)
- [Security](security.md)
- [Deployment](deployment.md)
- [The Yaiko Book](book.md)
- [Yaiko for Beginners](beginners-book.md)
