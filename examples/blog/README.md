# Yaiko Blog Example

A simple static blog built with Yaiko.

## Features

- 📝 Markdown posts loaded from `posts/` directory
- 🎨 Modern dark theme
- 🔍 SEO optimized (robots.txt, sitemap.xml)
- 📱 Responsive design
- 🧩 Uses Yaiko routing, static asset policy, structured logging, security headers, metadata, robots, and sitemap helpers

## Run

```bash
cd examples/blog
yaiko dev
```


Open [http://localhost:3000](http://localhost:3000)

## Add Posts

Create markdown files in `posts/`:

```markdown
# My Post Title
2026-01-15

Your content here...
```

Posts are sorted by date (newest first).

## Built-in modules demonstrated

The blog is intentionally small, but it shows how several Yaiko boundaries fit together: `Router` handles static and parameterized routes, `static_files` serves the public directory, `LoggingMiddleware` and `SecurityHeaders` protect the request pipeline, and the robots/sitemap endpoints provide SEO metadata. For the complete catalog, see [the built-in modules guide](../docs/content/built-in-modules.md).

Before submitting changes, run the repository verification sequence from the main [README](../../README.md): focused tests, formatting, strict Clippy, the feature matrix, CLI tests, and the blog/chat/auth example builds.

## Structure

```
blog/
├── src/main.rs       # Routes and post loading
├── posts/            # Markdown posts
├── public/css/       # Stylesheets
└── templates/        # HTML templates
```
