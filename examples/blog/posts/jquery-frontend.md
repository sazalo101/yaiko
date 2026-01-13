# Using jQuery with Yaiko
2026-01-08

Yaiko includes a powerful jQuery-based frontend toolkit.

## The Yaiko API Client

The core.js file provides a built-in API client:

```javascript
// GET request
Yaiko.api.get('/api/posts').then(function(data) {
    console.log(data.posts);
});

// POST request
Yaiko.api.post('/api/posts', {
    title: 'My Post',
    content: 'Hello world'
});
```

## Toast Notifications

Show beautiful notifications:

```javascript
Yaiko.ui.toast('Saved!', 'success');
Yaiko.ui.toast('Error occurred', 'error');
```

## Design System

Use the included CSS variables for consistent styling:

```css
.my-button {
    background: var(--color-primary);
    color: var(--color-text);
    border-radius: var(--radius-lg);
}
```

The frontend toolkit makes jQuery development feel modern again!
