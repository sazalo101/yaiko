# Security

Built-in security features in Yaiko.

## Security Middleware

Apply security middleware to your routes:

```rust
use yaiko_core::security::{SecurityHeaders, RateLimiter, CsrfProtection};

let router = Router::new()
    .get("/", home_handler)
    .use_middleware(SecurityHeaders::new())
    .use_middleware(RateLimiter::new(100, 60))
    .use_middleware(CsrfProtection::new());
```

## Security Headers

`SecurityHeaders` adds these headers to all responses:

| Header                   | Value                                      |
| ------------------------ | ------------------------------------------ |
| `X-Content-Type-Options` | `nosniff`                                  |
| `X-Frame-Options`        | `DENY`                                     |
| `X-XSS-Protection`       | `1; mode=block`                            |
| `Referrer-Policy`        | `strict-origin-when-cross-origin`          |
| `Permissions-Policy`     | `geolocation=(), microphone=(), camera=()` |

## Rate Limiting

Limit requests per IP address:

```rust
// 100 requests per 60 seconds
let rate_limiter = RateLimiter::new(100, 60);

router.use_middleware(rate_limiter);
```

When limit exceeded, returns `429 Too Many Requests`.

## CSRF Protection

Double-submit cookie pattern:

```rust
router.use_middleware(CsrfProtection::new());
```

**How it works:**
1. Safe methods (GET, HEAD, OPTIONS) → Sets CSRF cookie
2. Unsafe methods (POST, PUT, DELETE) → Validates token

**Frontend usage:**
```html
<meta name="csrf-token" content="{{ csrf_token }}">

<script>
// Include in AJAX requests
$.ajaxSetup({
    headers: {
        'X-CSRF-Token': $('meta[name="csrf-token"]').attr('content')
    }
});
</script>
```

## JWT Authentication

Use `JwtAuth` for stateless authentication:

```rust
use yaiko_core::auth::{JwtAuth, AuthMiddleware};
use std::sync::Arc;

let jwt = Arc::new(JwtAuth::new("your-secret-key"));

// Generate token on login
let token = jwt.generate_token("user123", vec!["admin".to_string()])?;

// Verify token
let claims = jwt.verify_token(&token)?;
println!("User: {}", claims.sub);

// Protect routes
let auth_middleware = AuthMiddleware::new(jwt.clone())
    .skip_path("/login")
    .skip_path("/register");

router.use_middleware(auth_middleware);
```

**Token in headers:**
```
Authorization: Bearer <token>
```

## Session Authentication

Use `SessionMiddleware` and `SessionAuth` for fullstack app login flows:

```rust
use std::sync::Arc;
use yaiko_core::{
    MemorySessionStore, SessionAuth, SessionMiddleware,
    login_session, logout_session,
};

let session_store = Arc::new(MemorySessionStore::new());

let router = router
    .use_middleware(SessionAuth::new()
        .skip_path("/")
        .skip_path("/login")
        .skip_path("/register"))
    .use_middleware(SessionMiddleware::new(session_store).secure(false));
```

Log a user in by writing to the session and rotating the session ID:

```rust
use yaiko_core::login_session;

let session = req.session.as_ref().expect("session middleware required");
login_session(session, "user-123", &vec!["admin".to_string()])?;
```

Log out by destroying the session:

```rust
use yaiko_core::logout_session;

if let Some(session) = &req.session {
    logout_session(session);
}
```

Protect role-based routes:

```rust
use yaiko_core::require_role;

if let Err(response) = require_role(&req, "admin") {
    return Ok(response);
}
```

## CORS

Configure Cross-Origin Resource Sharing:

```rust
use yaiko_core::middleware::Cors;

let cors = Cors::new()
    .allow_origin("https://example.com")
    .allow_methods("GET, POST, PUT, DELETE")
    .allow_headers("Content-Type, Authorization")
    .max_age(3600);

router.use_middleware(cors);
```

## Password Hashing

Use the built-in helper functions for password hashing:

```rust
use yaiko_core::{hash_password, verify_password};

// Hash password
let password_hash = hash_password("user_password")?;

// Verify password
let valid = verify_password("user_password", &password_hash)?;
```

Or use bcrypt directly:

```rust
use yaiko_core::bcrypt::{hash, verify, DEFAULT_COST};

let password_hash = hash("user_password", DEFAULT_COST)?;
let valid = verify("user_password", &password_hash)?;
```

## Configuration

`yaiko.toml`:
```toml
[security]
cors_origins = ["https://myapp.com", "https://admin.myapp.com"]
rate_limit_requests = 100
rate_limit_window_secs = 60
csrf_enabled = true
```

## Best Practices

1. **Use HTTPS in production** — Set `secure: true` on cookies
2. **Rotate JWT secrets** — Use environment variables
3. **Validate all input** — Never trust user data
4. **Use parameterized queries** — Prevent SQL injection
5. **Set Content-Security-Policy** — Add to `SecurityHeaders`
