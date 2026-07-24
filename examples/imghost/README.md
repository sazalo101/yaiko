# ImgHost

A modern, free image hosting web application built with the **Yaiko** framework.

🌐 **Live Demo**: [https://imghost.se](https://imghost.se)

## Features

- **Drag & Drop Upload** — Drop or browse images directly from the landing page
- **Private Uploads** — Uploaded images are not shown publicly; each image only accessible via its direct link
- **NSFW Content Moderation** — Integrates JigsawStack NSFW API to auto-reject nude or explicit content
- **Viewer Page** — `/i/:id` displays image metadata, view count, and embed snippets (HTML, Markdown)
- **Delete Token** — Each upload gets a secret token for authorized deletion
- **Multiple formats** — Supports JPEG, PNG, GIF, WebP, SVG, BMP up to 10 MB

## Quick Start

```bash
# 1. Clone the Yaiko repository
git clone https://github.com/sazalo101/yaiko.git
cd yaiko/examples/imghost

# 2. Run database migrations
sqlite3 imghost.db < migrations/001_init.sql

# 3. Copy and configure environment
cp .env.example .env
# Edit .env and set JIGSAWSTACK_API_KEY for NSFW moderation

# 4. Start dev server
cargo run --manifest-path ../../yaiko-cli/Cargo.toml -- dev

# Or run directly
cargo run
```

## Environment Variables

| Variable                | Description                                             | Default                   |
|-------------------------|---------------------------------------------------------|---------------------------|
| `HOST`                  | Server bind host                                        | `127.0.0.1`               |
| `PORT`                  | Server port                                             | `3000`                    |
| `DATABASE_URL`          | SQLite connection string                                | `sqlite://imghost.db`     |
| `SITE_URL`              | Public base URL (used for absolute links and NSFW scan) | `http://localhost:3000`   |
| `JIGSAWSTACK_API_KEY`   | API key for NSFW content moderation                     | *(optional)*              |

## NSFW Moderation

This example integrates [JigsawStack](https://jigsawstack.com/docs/api-reference/validate/nsfw) to scan images:
- If an image is flagged as **nsfw**, **nudity**, or has score > 40%, the upload is rejected and the file is deleted immediately.
- Get your API key at [jigsawstack.com](https://jigsawstack.com).

## Project Structure

```
imghost/
├── migrations/
│   └── 001_init.sql       # SQLite schema
├── public/
│   ├── css/style.css      # Cream-white theme
│   ├── js/app.js          # Upload & results UI
│   ├── js/viewer.js       # Viewer page logic
│   ├── index.html         # Landing page
│   └── viewer.html        # Image viewer SPA
└── src/
    ├── main.rs            # App entry point & routing
    ├── models/image.rs    # Data types
    └── routes/
        ├── upload.rs      # Multipart upload + NSFW check
        ├── image.rs       # GET/DELETE image metadata
        └── gallery.rs     # (private) gallery endpoint
```

## Production Deployment

See the [Yaiko deployment guide](https://github.com/sazalo101/yaiko/blob/main/docs/deployment.md).

```bash
cargo build --release
# Copy target/release/imghost + public/ + migrations/ to server
# Set up systemd + Nginx + Certbot SSL
```
