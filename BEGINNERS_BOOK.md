# Yaiko for Beginners

> A friendly, step-by-step guide to building web applications using Rust and jQuery — written for beginners.  
> 🌐 **Repository**: [github.com/sazalo101/yaiko](https://github.com/sazalo101/yaiko)

---

## Table of Contents

1. [Chapter 1 — What is Web Development?](#chapter-1-what-is-web-development)
2. [Chapter 2 — Installation & Your First Web App in 3 Minutes](#chapter-2-installation--your-first-web-app-in-3-minutes)
3. [Chapter 3 — Understanding Your Project Files](#chapter-3-understanding-your-project-files)
4. [Chapter 4 — How Web Requests Work](#chapter-4-how-web-requests-work)
5. [Chapter 5 — Building Pages with HTML & Templates](#chapter-5-building-pages-with-html--templates)
6. [Chapter 6 — Adding Interactivity with jQuery](#chapter-6-adding-interactivity-with-jquery)
7. [Chapter 7 — Saving Data with SQLite](#chapter-7-saving-data-with-sqlite)
8. [Chapter 8 — Handling Forms & User Input](#chapter-8-handling-forms--user-input)
9. [Chapter 9 — Full Beginner Project: Build a Quick Note App](#chapter-9-full-beginner-project-build-a-quick-note-app)
10. [Chapter 10 — Passwords & User Accounts](#chapter-10-passwords--user-accounts)
11. [Chapter 11 — Real-Time Updates Explained Simply](#chapter-11-real-time-updates-explained-simply)
12. [Chapter 12 — Putting Your Web App Online](#chapter-12-putting-your-web-app-online)
13. [Chapter 13 — Beginner Cheat Sheet](#chapter-13-beginner-cheat-sheet)

---

## Chapter 1 — What is Web Development?

When you visit a website like `google.com` or `youtube.com`, two computers talk to each other:

1. **The Client (Your Web Browser)**: The computer showing you pages, images, and buttons.
2. **The Server (The Backend)**: A computer sitting far away that receives requests, reads from a database, and sends back web pages or JSON data.

```
+------------------+     1. Sends HTTP Request      +------------------+
|                  | -----------------------------> |                  |
|   Web Browser    |                                |   Yaiko Server   |
|     (Client)     | <----------------------------- |    (Backend)     |
+------------------+      2. Returns HTML / Data    +------------------+
```

### Why Learn Yaiko?

If you are coming from Python or JavaScript, you might be used to web frameworks like Django, Flask, or Express. 

Yaiko gives you the **simplicity of Python frameworks** combined with the **unmatched speed and memory efficiency of Rust**:

- **Fast & Lightweight**: Starts in milliseconds and uses almost no RAM.
- **Batteries-Included**: Routing, databases, security, and templates work out of the box.
- **No Complex Build Steps**: You don't need React, Webpack, or NPM. Just Rust on the server and clean HTML + jQuery in the browser.

---

## Chapter 2 — Installation & Your First Web App in 3 Minutes

### Prerequisites

To get started, you only need Rust installed on your computer.

If you don't have Rust yet, install it by visiting [rustup.rs](https://rustup.rs) or running:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Step 1: Install the Yaiko CLI

The Yaiko CLI (Command Line Interface) helps you create new projects and run your development server.

```bash
git clone https://github.com/sazalo101/yaiko.git
cd yaiko
cargo install --path yaiko-cli --force
```

Verify your installation:

```bash
yaiko doctor
```

---

### Step 2: Create Your First Project

Let's create an app called `hello-yaiko`:

```bash
yaiko init hello-yaiko -d sqlite
cd hello-yaiko
```

---

### Step 3: Start the Development Server

Run the development command:

```bash
yaiko dev
```

You will see output like this:

```
    ██╗   ██╗ █████╗ ██╗██╗  ██╗ ██████╗ 
    ╚██╗ ██╔╝██╔══██╗██║██║ ██╔╝██╔═══██╗
     ╚████╔╝ ███████║██║█████╔╝ ██║   ██║
      ╚██╔╝  ██╔══██║██║██╔═██╗ ██║   ██║
       ██║   ██║  ██║██║██║  ██╗╚██████╔╝
       ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝ ╚═════╝ 
    
[*] Starting dev server...
[OK] Server running at http://127.0.0.1:3000
```

Open your browser and navigate to `http://127.0.0.1:3000`. Congratulations! Your first Yaiko web app is officially running!

---

## Chapter 3 — Understanding Your Project Files

When you open your new `hello-yaiko` folder in an editor (like VS Code), you will see these files:

```
hello-yaiko/
├── src/
│   └── main.rs         # The brain of your app (Rust code)
├── templates/
│   └── index.html      # What the user sees in the browser
├── public/
│   ├── css/
│   │   └── main.css    # Colors and styles (Cream theme)
│   └── js/
│       ├── core.js     # Yaiko helper functions
│       └── app.js      # Your interactive frontend code
├── yaiko.toml          # App configuration (port, title, settings)
├── .env                # Secret keys and database settings
└── Cargo.toml          # Rust package manager setup
```

### Line-by-Line Breakdown of `src/main.rs`

Open `src/main.rs`. This is where all backend logic lives:

```rust
use yaiko_core::{App, Router, Server, Request, Response, BoxError, json};

// 1. A function that handles web requests
async fn home_handler(_req: Request) -> Result<Response, BoxError> {
    Ok(Response::new().html("<h1>Hello from Yaiko!</h1>"))
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    // 2. Define web routes (URLs)
    let router = Router::new()
        .get("/", home_handler);

    // 3. Create the application
    let app = App::new().router(router);

    // 4. Start the server on port 3000
    Server::new(app, "127.0.0.1:3000".parse()?).run().await?;
    Ok(())
}
```

---

## Chapter 4 — How Web Requests Work

Web browsers send different types of **HTTP Requests**:

- **`GET`**: "Please give me a page or piece of data." (e.g., viewing a profile or homepage)
- **`POST`**: "Here is new data, please save it." (e.g., submitting a form or logging in)
- **`DELETE`**: "Please remove this item."

### Creating Routes in Yaiko

Routes connect URLs to Rust handler functions:

```rust
let router = Router::new()
    .get("/", home_handler)
    .get("/about", about_handler)
    .get("/api/greet", greet_handler)
    .post("/api/save", save_handler);
```

### Reading URL Parameters

If a URL is `/users/42`, you can read the number `42` like this:

```rust
// URL: /users/:id
async fn get_user(req: Request) -> Result<Response, BoxError> {
    let user_id = req.param("id").unwrap_or("0");
    Ok(Response::new().json(&json!({
        "user_id": user_id,
        "status": "active"
    }))?)
}
```

### Reading Query Parameters

If a URL is `/search?q=rust`, you can read `q`:

```rust
// URL: /search?q=rust
async fn search(req: Request) -> Result<Response, BoxError> {
    let query = req.query("q").unwrap_or("nothing".into());
    Ok(Response::new().text(format!("Searching for: {}", query)))
}
```

---

## Chapter 5 — Building Pages with HTML & Templates

Instead of sending raw text, web apps send HTML pages.

In Yaiko, HTML files live in the `templates/` folder.

### Step 1: Edit `templates/index.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>My Personal Web App</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
    <main class="main">
        <h1>Welcome to My Website</h1>
        <p>Built with Rust + Yaiko!</p>
        <button id="click-me-btn" class="btn btn--primary">Click Me</button>
    </main>

    <script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>
    <script src="/static/js/app.js"></script>
</body>
</html>
```

### Step 2: Serve the Template from `src/main.rs`

```rust
async fn index_handler(_req: Request) -> Result<Response, BoxError> {
    let html = std::fs::read_to_string("templates/index.html")?;
    Ok(Response::new().html(html))
}
```

---

## Chapter 6 — Adding Interactivity with jQuery

Instead of learning heavy frameworks like React or Angular, Yaiko uses **jQuery** for quick, simple interactive features.

Open `public/js/app.js`:

```javascript
$(document).ready(function() {
    console.log("App ready!");

    // Listen for button click
    $('#click-me-btn').on('click', function() {
        alert("You clicked the button!");
    });
});
```

### Making an AJAX Request to the Backend

AJAX lets your web page talk to the backend server without reloading the page.

#### Backend (`src/main.rs`):
```rust
async fn random_number_handler(_req: Request) -> Result<Response, BoxError> {
    let number = rand::random::<u8>();
    Ok(Response::new().json(&json!({ "number": number }))?)
}
```

#### Frontend (`public/js/app.js`):
```javascript
$('#get-number-btn').on('click', function() {
    $.ajax({
        url: '/api/random',
        method: 'GET',
        success: function(response) {
            $('#result-box').text("Random Number: " + response.number);
        }
    });
});
```

---

## Chapter 7 — Saving Data with SQLite

A web app needs a database to remember things (like user posts, comments, or settings).

Yaiko supports **SQLite**, which stores all your data in a single file (`data.db`) on your computer.

### Step 1: Create a Database Migration

Migrations define the structure of your database tables:

```bash
yaiko migrate create create_notes_table
```

Edit the file generated in `migrations/`:

```sql
CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Step 2: Apply the Migration

Run the migrate command:

```bash
yaiko migrate run
```

Output:
```
[*] Running pending migrations...
  [*] Running: 20260725_create_notes_table.sql
[OK] Applied 1 migration(s).
```

---

## Chapter 8 — Handling Forms & User Input

When a user submits a form (like a search box or sign-up form), you want to validate the input before saving it.

### Validating Input in Rust

```rust
use yaiko_core::validation::{Validator, Required, MinLength, MaxLength};

async fn save_note_handler(mut req: Request) -> Result<Response, BoxError> {
    // 1. Read form fields
    let form = req.form_data().await?;

    // 2. Define validation rules
    let validator = Validator::new()
        .add_rule("title", Required)
        .add_rule("title", MinLength(3))
        .add_rule("title", MaxLength(50))
        .add_rule("content", Required);

    // 3. Check for errors
    if let Err(errors) = validator.validate(&form) {
        return Ok(Response::new()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .json(&json!({ "status": "error", "errors": errors }))?);
    }

    let title = form.get("title").unwrap();
    let content = form.get("content").unwrap();

    Ok(Response::new().json(&json!({
        "status": "success",
        "title": title,
        "content": content
    }))?)
}
```

---

## Chapter 9 — Full Beginner Project: Build a Quick Note App

Let's combine everything we learned into a complete, working Quick Note Application!

### Project Blueprint

- `GET /` — Serves the homepage with note creation form and list.
- `GET /api/notes` — Returns all saved notes as JSON.
- `POST /api/notes` — Saves a new note to the database.

---

### Step 1: Backend (`src/main.rs`)

```rust
use yaiko_core::{
    App, Router, Server, Request, Response, BoxError, json, StatusCode
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Serialize, Deserialize)]
struct Note {
    id: usize,
    title: String,
    content: String,
}

struct AppState {
    notes: Mutex<Vec<Note>>,
}

async fn index_handler(_req: Request) -> Result<Response, BoxError> {
    let html = std::fs::read_to_string("templates/index.html")?;
    Ok(Response::new().html(html))
}

async fn list_notes(req: Request) -> Result<Response, BoxError> {
    let state = req.app_data::<Arc<AppState>>().unwrap();
    let notes = state.notes.lock().await;
    Ok(Response::new().json(&*notes)?)
}

async fn create_note(mut req: Request) -> Result<Response, BoxError> {
    let state = req.app_data::<Arc<AppState>>().unwrap();
    let body = req.json().await?;

    let title = body["title"].as_str().unwrap_or("Untitled").to_string();
    let content = body["content"].as_str().unwrap_or("").to_string();

    let mut notes = state.notes.lock().await;
    let new_id = notes.len() + 1;
    let new_note = Note { id: new_id, title, content };

    notes.push(new_note.clone());

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&new_note)?)
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let state = Arc::new(AppState {
        notes: Mutex::new(vec![
            Note { id: 1, title: "Welcome Note".into(), content: "My first note in Yaiko!".into() }
        ]),
    });

    let router = Router::new()
        .static_files("/static", "./public")
        .get("/", index_handler)
        .get("/api/notes", list_notes)
        .post("/api/notes", create_note);

    let mut app = App::new().router(router);
    app.data(state);

    println!("Server running on http://127.0.0.1:3000");
    Server::new(app, "127.0.0.1:3000".parse()?).run().await?;
    Ok(())
}
```

---

### Step 2: HTML Interface (`templates/index.html`)

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Quick Notes</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
    <div class="app">
        <header class="header">
            <nav class="nav">
                <div class="nav__brand">
                    <span class="logo-mark">N</span>
                    <span class="brand-name">QuickNotes</span>
                </div>
            </nav>
        </header>

        <main class="main">
            <section style="max-width: 600px; margin: 0 auto;">
                <h2>Create a Note</h2>
                <form id="note-form" style="margin-bottom: 2rem;">
                    <div style="margin-bottom: 1rem;">
                        <input type="text" id="title-input" placeholder="Title" required style="width: 100%; padding: 0.5rem;">
                    </div>
                    <div style="margin-bottom: 1rem;">
                        <textarea id="content-input" placeholder="Note content..." required style="width: 100%; padding: 0.5rem; height: 80px;"></textarea>
                    </div>
                    <button type="submit" class="btn btn--primary">Save Note</button>
                </form>

                <h2>My Notes</h2>
                <div id="notes-list"></div>
            </section>
        </main>
    </div>

    <script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>
    <script>
    $(document).ready(function() {
        // Load initial notes
        loadNotes();

        function loadNotes() {
            $.get('/api/notes', function(notes) {
                $('#notes-list').empty();
                notes.forEach(function(note) {
                    $('#notes-list').append(`
                        <div class="feature-card" style="margin-bottom: 1rem;">
                            <h3>${note.title}</h3>
                            <p>${note.content}</p>
                        </div>
                    `);
                });
            });
        }

        // Handle note creation
        $('#note-form').on('submit', function(e) {
            e.preventDefault();
            const data = {
                title: $('#title-input').val(),
                content: $('#content-input').val()
            };

            $.ajax({
                url: '/api/notes',
                method: 'POST',
                contentType: 'application/json',
                data: JSON.stringify(data),
                success: function() {
                    $('#title-input').val('');
                    $('#content-input').val('');
                    loadNotes();
                }
            });
        });
    });
    </script>
</body>
</html>
```

---

## Chapter 10 — Passwords & User Accounts

Security is extremely important when users sign up or log in.

Never store raw passwords in a database! Always hash them.

### Hashing Passwords in Yaiko

```rust
use yaiko_core::{hash_password, verify_password};

// 1. When a user registers:
let user_password = "my_secret_password_123";
let hashed = hash_password(user_password)?;
// Save `hashed` into database!

// 2. When a user logs in:
let is_correct = verify_password("my_secret_password_123", &hashed)?;
if is_correct {
    println!("Login successful!");
} else {
    println!("Wrong password!");
}
```

---

## Chapter 11 — Real-Time Updates Explained Simply

Usually, a web browser must ask the server for updates (`GET` requests).

With **WebSockets**, the server can push messages to the browser instantly when something happens!

### Example: Live Chat or Notification Badge

```javascript
// Connect to WebSocket server
const socket = new WebSocket("ws://127.0.0.1:3000/ws");

// Receive real-time messages
socket.onmessage = function(event) {
    const data = JSON.parse(event.data);
    alert("New alert: " + data.message);
};
```

---

## Chapter 12 — Putting Your Web App Online

When your app works on your computer (`http://127.0.0.1:3000`), you are ready to publish it so anyone on the internet can access it.

### Step 1: Build the Production Binary

```bash
yaiko build --release
```

This compiles your entire application into a fast, single executable file in `target/release/hello-yaiko`.

### Step 2: Copy to Your Linux VPS Server

```bash
scp target/release/hello-yaiko user@your-server-ip:/opt/hello-yaiko/
scp -r public templates user@your-server-ip:/opt/hello-yaiko/
```

### Step 3: Run with Systemd

Create `/etc/systemd/system/hello-yaiko.service`:

```ini
[Unit]
Description=Hello Yaiko Web Application
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/hello-yaiko
ExecStart=/opt/hello-yaiko/hello-yaiko
Restart=always

[Install]
WantedBy=multi-user.target
```

Enable and start your app:

```bash
systemctl enable --now hello-yaiko
```

Your app is now live 24/7!

---

## Chapter 13 — Beginner Cheat Sheet

### Common CLI Commands

| Command | What It Does |
|---|---|
| `yaiko init my-app` | Creates a new web project |
| `yaiko dev` | Starts the live development server |
| `yaiko build --release` | Compiles your app for production |
| `yaiko migrate create <name>` | Creates a database migration file |
| `yaiko migrate run` | Applies all database migrations |

### Common Rust Response Helpers

```rust
Response::new().html("<h1>Title</h1>")      // HTML response
Response::new().json(&data)?                // JSON API response
Response::new().text("Plain text")          // Plain text response
Response::new().redirect("/login")          // Page redirect (302)
Response::no_content()                      // Empty success response (204)
```

---

## Conclusion

Congratulations on taking your first steps with **Yaiko**! 

You now know how web servers work, how to build web pages with HTML and jQuery, how to save data in SQLite databases, and how to put your code live on the web.

For deeper architectural topics, performance benchmarks, and advanced project ideas, read **[The Yaiko Book](BOOK.md)**.
