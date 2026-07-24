# Yaiko Blog Example

A simple static blog built with Yaiko.

## Features

- 📝 Markdown posts loaded from `posts/` directory
- 🎨 Modern dark theme
- 🔍 SEO optimized (robots.txt, sitemap.xml)
- 📱 Responsive design

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

## Structure

```
blog/
├── src/main.rs       # Routes and post loading
├── posts/            # Markdown posts
├── public/css/       # Stylesheets
└── templates/        # HTML templates
```
