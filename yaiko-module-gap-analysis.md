# Yaiko Built-in Module Gap Analysis

## Executive assessment

Yaiko already contains broad foundations for HTTP routing, templates, static files, sessions, middleware, storage, SQLx database access, validation, caching, authentication, cryptography, rate limiting, webhooks, uploads, background tasks, metrics, tracing, health checks, media editing, collaboration, and deployment configuration. The requested catalog therefore contains many capabilities that are present under different names, while the largest gaps are frontend primitives, HTTP/2 and network/TLS helpers, SSR/SSG/ISR/hydration, query-client ergonomics, RBAC, RPC/proxy, pub/sub/event buses, HMR/watch/type generation, rollback/SSL/Docker tooling, feed/robots, and persistent implementations of several in-memory media stores.

| Catalog area | Present or partial Yaiko coverage | Missing or materially incomplete | Priority |
|---|---|---|---|
| Runtime core | `request`, `response`, `router`, `websocket`, `http_client`, `server` | Explicit HTTP/2, network, TLS, cluster, worker, and stream abstractions | P0 |
| Routing/rendering | `router`, `template`, `static_files`, `middleware` | Filesystem router, SSR, SSG, ISR, hydration, layouts, bundler | P0 |
| Frontend primitives | `static_files`, `seo`, `subtitle_style` | Image, font, form, link, script, head, metadata, style, icon frontend APIs | P0 |
| Data layer | `database`, `migrations`, `cache`, `validation`, `data_transfer`, `resilience` | ORM, pool abstraction, transaction helpers, query client, serialization facade | P0 |
| Security | `encryption`, `auth`, `cors`, `rate_limit`, `security`, `csp`, `media_access` | JWT is embedded in auth rather than isolated; RBAC and unified headers policy are incomplete | P0 |
| API layer | `router`, `extract`, `openapi`, `webhook`, `file_upload`, `resumable_upload` | Explicit API facade, RPC, reverse proxy | P0 |
| Background/realtime | `jobs`, `task_scheduler`, `websocket`, `ws_channels`, `notifications`, `delivery` | Queue abstraction, pub/sub, typed event bus, unified notify facade | P1 |
| Dev tooling | `yaiko-cli`, `testing`, `logging`, `openapi` | HMR, file watch, lint/format wrappers, type generation | P1 |
| Observability | `audit`, `metrics`, `metrics_registry`, `tracing_context`, `health`, `error` | Unified log facade and observability export facade | P1 |
| Deployment | `config`, `server`, `static_files`, `storage` | Deploy, rollback, SSL, Docker orchestration modules | P1 |
| Utilities | `http_client`, `media_processing`, `compression`, `i18n`, `seo` | URL/path facades, sitemap is partial, RSS/Atom feed, robots.txt | P1 |
| Media/editor | Extensive media modules and seven recent collaboration/workflow batches | Persistent repositories, HTTP/WebSocket application APIs, browser editor integration, worker execution | P0 |

## Recommended implementation order

The first implementation batch should establish the runtime and application boundary: HTTP/2/network/TLS helpers, stream and worker abstractions, filesystem routing, RBAC, API/RPC/proxy facades, frontend head/metadata/form/link/script primitives, and utility URL/path/robots/feed modules. The second batch should add persistent data ergonomics, queue/pub-sub/events, developer tooling, deployment helpers, and observability facades. The media modules should then be connected to SQLx repositories, HTTP/WebSocket routes, controlled FFmpeg workers, and the browser editor.

The existing modules should not be duplicated under cosmetic names. For example, `auth.rs` already contains JWT functionality, `openapi.rs` already covers OpenAPI generation, `webhook.rs` and `media_webhook.rs` cover webhook signing, `file_upload.rs` and `resumable_upload.rs` cover uploads, and `media_delivery_policy.rs` covers substantial CORS/range/header behavior. New modules should provide missing abstractions or compatibility facades over these implementations.
