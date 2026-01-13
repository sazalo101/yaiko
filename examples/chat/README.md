# Yaiko Chat Example

An AI chat application built with Yaiko using OpenRouter API.

## Features

- User authentication (email/password)
- Real-time AI chat using OpenRouter API
- Conversation history
- Claude-like dark theme UI
- robots.txt and sitemap.xml

## Setup

1. Set your OpenRouter API key in `.env`:
```bash
OPENROUTER_API_KEY=your-api-key-here
```

2. Run:
```bash
cargo run
# or
yaiko dev
```

3. Open http://localhost:3000

## API Endpoints

### Auth
| Method | Endpoint      | Description      |
| ------ | ------------- | ---------------- |
| POST   | `/api/signup` | Create account   |
| POST   | `/api/login`  | Login            |
| POST   | `/api/logout` | Logout           |
| GET    | `/api/me`     | Get current user |

### Chat
| Method | Endpoint                 | Description        |
| ------ | ------------------------ | ------------------ |
| POST   | `/api/chat`              | Send message       |
| GET    | `/api/conversations`     | List conversations |
| GET    | `/api/conversations/:id` | Get conversation   |

### SEO
| Method | Endpoint       | Description     |
| ------ | -------------- | --------------- |
| GET    | `/robots.txt`  | SEO robots file |
| GET    | `/sitemap.xml` | SEO sitemap     |

## Structure

```
chat/
├── src/main.rs           # Server + API
├── templates/
│   ├── index.html        # Login/Signup
│   └── chat.html         # Chat interface
├── public/
│   ├── css/app.css       # Styles
│   └── js/
│       ├── auth.js       # Auth logic
│       └── chat.js       # Chat logic
├── .env                  # API key
└── yaiko.toml            # Config
```
