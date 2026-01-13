# Yaiko Notes Example

A real-world note-taking application built with Yaiko.

## Features

- Create, read, update, delete notes
- In-memory storage (no database required)
- Real-time search
- Modern dark theme UI
- Responsive design

## Run

```bash
cd examples/notes
cargo run
```

Open [http://localhost:3000](http://localhost:3000)

Or with yaiko CLI:
```bash
yaiko dev
```

## API Endpoints

| Method | Endpoint         | Description       |
| ------ | ---------------- | ----------------- |
| GET    | `/api/notes`     | List all notes    |
| GET    | `/api/notes/:id` | Get a single note |
| POST   | `/api/notes`     | Create a note     |
| PUT    | `/api/notes/:id` | Update a note     |
| DELETE | `/api/notes/:id` | Delete a note     |

## Structure

```
notes/
├── src/main.rs           # Server + API routes
├── templates/index.html  # UI template
├── public/
│   ├── css/app.css       # Styles
│   └── js/app.js         # jQuery client
└── yaiko.toml            # Config
```
