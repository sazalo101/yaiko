# Database

Work with PostgreSQL or SQLite in Yaiko.

## Setup

### PostgreSQL
```bash
# .env
DATABASE_URL=postgres://user:password@localhost:5432/myapp
```

### SQLite
```bash
# .env
DATABASE_URL=sqlite:./data.db?mode=rwc
```

## Migrations

Create and run migrations:

```bash
# Create a migration
yaiko migrate create users

# Run pending migrations
yaiko migrate run

# Check migration status
yaiko migrate status

# Rollback (requires sqlx CLI)
sqlx migrate revert
```

### Migration File

`migrations/20260112_users.sql`:
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

## Models

Generate a model:

```bash
yaiko generate model user
```

`src/models/user.rs`:
```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

impl User {
    pub async fn all(pool: &sqlx::PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
    }

    pub async fn find(pool: &sqlx::PgPool, id: i32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_email(pool: &sqlx::PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &sqlx::PgPool, data: CreateUser) -> Result<Self, sqlx::Error> {
        let password_hash = bcrypt::hash(&data.password, bcrypt::DEFAULT_COST)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO users (email, password_hash, name, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            RETURNING *
            "#
        )
        .bind(&data.email)
        .bind(&password_hash)
        .bind(&data.name)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &sqlx::PgPool, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
```

## Database Connection

Initialize the database pool:

```rust
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL")?;
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;
    
    // Use pool in handlers...
    Ok(())
}
```

## Using in Controllers

```rust
use crate::models::User;

pub async fn list(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = req.state::<sqlx::PgPool>()?;
    let users = User::all(pool).await?;
    
    Ok(Response::new().json(&json!({ "users": users }))?)
}

pub async fn create(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = req.state::<sqlx::PgPool>()?;
    let data: CreateUser = req.json().await?;
    
    let user = User::create(pool, data).await?;
    
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "user": user }))?)
}
```
