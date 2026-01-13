# Frontend

Build frontends with jQuery and Handlebars templates.

## Overview

Yaiko uses:
- **Handlebars** — Server-side templating
- **jQuery** — Client-side interactions
- **CSS Variables** — Design system

## Templates

`templates/index.html`:
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="csrf-token" content="{{ csrf_token }}">
    <title>{{ title }}</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
    <div class="app">
        <header class="header">
            <h1>{{ heading }}</h1>
        </header>
        
        <main class="main">
            {{ content }}
        </main>
    </div>
    
    <div id="toast-container"></div>
    
    <script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>
    <script src="/static/js/core.js"></script>
    <script src="/static/js/app.js"></script>
</body>
</html>
```

## Rendering Templates

```rust
use yaiko_core::template::TemplateEngine;

let engine = TemplateEngine::new("./templates")?;

async fn home(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let html = engine.render("index.html", &json!({
        "title": "My App",
        "heading": "Welcome!",
        "content": "<p>Hello, World!</p>"
    }))?;
    
    Ok(Response::new().html(&html))
}
```

## jQuery API Client

`public/js/core.js` provides `Yaiko.api`:

```javascript
// GET request
Yaiko.api.get('/api/users')
    .then(function(data) {
        console.log(data.users);
    });

// POST request
Yaiko.api.post('/api/users', {
    name: 'John',
    email: 'john@example.com'
}).then(function(data) {
    Yaiko.ui.toast('User created!', 'success');
});

// PUT request
Yaiko.api.put('/api/users/123', { name: 'Jane' });

// DELETE request
Yaiko.api.delete('/api/users/123');
```

**CSRF tokens are automatically included.**

## UI Components

### Toast Notifications
```javascript
Yaiko.ui.toast('Operation successful', 'success');
Yaiko.ui.toast('Warning message', 'warning');
Yaiko.ui.toast('Error occurred', 'error');
Yaiko.ui.toast('Info message', 'info');
```

### Loading States
```javascript
Yaiko.ui.showLoading($('#submit-btn'));
// After operation
Yaiko.ui.hideLoading($('#submit-btn'));
```

### Modals
```javascript
Yaiko.ui.modal.open('my-modal');
Yaiko.ui.modal.close('my-modal');
```

## CSS Design System

`public/css/main.css` includes CSS variables:

```css
:root {
    /* Colors */
    --color-primary: #6366f1;
    --color-secondary: #10b981;
    --color-bg: #0f172a;
    --color-text: #f1f5f9;
    --color-text-muted: #94a3b8;
    
    /* Typography */
    --font-sans: "Inter", sans-serif;
    --font-mono: "JetBrains Mono", monospace;
    
    /* Spacing */
    --space-4: 1rem;
    --space-8: 2rem;
    
    /* Border Radius */
    --radius-lg: 0.75rem;
    
    /* Shadows */
    --shadow-glow: 0 0 20px rgba(99, 102, 241, 0.3);
}
```

### Button Styles
```html
<button class="btn btn--primary">Primary</button>
<button class="btn btn--secondary">Secondary</button>
```

### Glassmorphism
```html
<div class="glass">
    Frosted glass effect
</div>
```

## Static Files

Files in `public/` are served at `/static/`:

```
public/
├── css/
│   └── main.css      → /static/css/main.css
├── js/
│   ├── core.js       → /static/js/core.js
│   └── app.js        → /static/js/app.js
├── images/
│   └── logo.png      → /static/images/logo.png
└── fonts/
```

## Example: Todo App

```html
<div id="todo-app">
    <input type="text" id="new-todo" placeholder="Add todo...">
    <button id="add-btn" class="btn btn--primary">Add</button>
    <ul id="todo-list"></ul>
</div>

<script>
$(document).ready(function() {
    function loadTodos() {
        Yaiko.api.get('/api/todos').then(function(data) {
            $('#todo-list').empty();
            data.todos.forEach(function(todo) {
                $('#todo-list').append(
                    '<li data-id="' + todo.id + '">' + 
                    todo.title + 
                    ' <button class="delete-btn">×</button></li>'
                );
            });
        });
    }
    
    $('#add-btn').on('click', function() {
        var title = $('#new-todo').val();
        Yaiko.api.post('/api/todos', { title: title })
            .then(function() {
                $('#new-todo').val('');
                loadTodos();
                Yaiko.ui.toast('Todo added!', 'success');
            });
    });
    
    $(document).on('click', '.delete-btn', function() {
        var id = $(this).parent().data('id');
        Yaiko.api.delete('/api/todos/' + id)
            .then(function() {
                loadTodos();
            });
    });
    
    loadTodos();
});
</script>
```
