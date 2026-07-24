# ImgHost — wrk Performance Benchmark Report

> **Target App**: ImgHost ([https://imghost.se](https://imghost.se))  
> **Framework**: Yaiko (Rust + Hyper + Tokio)  
> **Database**: SQLite (WAL mode, 20 max connections)  
> **Reverse Proxy**: Nginx with Let's Encrypt TLS/SSL  
> **Tool Used**: `wrk 4.1.0`  
> **Date**: July 2026

---

## 📊 Executive Summary

Following backend optimizations (SQLite WAL journal mode and asynchronous non-blocking view count updates), **ImgHost** achieves:
- **11,234.57 req/sec** raw HTTP performance with **8.48 ms** median latency.
- **3,053.51 req/sec** over production Nginx SSL encryption (`https://imghost.se`).
- **12,042.80 req/sec** under a 1,000 concurrent connection stress test with **zero crashes** and **zero timeouts**.
- Full recovery of the `/api/images/:id` GET endpoint from a `0.5 req/sec` write-lock collapse to **60.46 req/sec**.

---

## 🚀 Server-Side Benchmark Results (`wrk` on Production Host)

### 1. Direct Engine Performance (`http://127.0.0.1:3000/`)
```bash
wrk -t4 -c100 -d30s --latency http://127.0.0.1:3000/
```

| Metric | Value |
|--------|-------|
| **Requests/sec** | **11,234.57** |
| **Transfer Rate** | **57.82 MB/sec** |
| **p50 Latency** | **8.48 ms** |
| **p75 Latency** | **9.32 ms** |
| **p90 Latency** | **10.71 ms** |
| **p99 Latency** | **19.07 ms** |
| Total Requests | **337,247** (in 30s) |
| Socket Errors | **0** |
| Timeouts | **0** |

---

### 2. Nginx SSL Reverse Proxy (`https://imghost.se/`)
```bash
wrk -t4 -c100 -d30s --latency https://imghost.se/
```

| Metric | Value |
|--------|-------|
| **Requests/sec** | **3,053.51** |
| **Transfer Rate** | **15.83 MB/sec** |
| **p50 Latency** | **32.03 ms** |
| **p75 Latency** | **33.89 ms** |
| **p90 Latency** | **36.03 ms** |
| **p99 Latency** | **44.07 ms** |
| Total Requests | **91,705** (in 30s) |
| Socket Errors | **0** |
| Timeouts | **0** |

---

### 3. API Endpoint Pre-Fix vs. Post-Fix (`/api/images/:id`)
```bash
wrk -t4 -c100 -d30s --latency http://127.0.0.1:3000/api/images/9oMxc8b5
```

| Metric | Pre-Optimization | Post-Optimization | Status |
|--------|------------------|-------------------|--------|
| **Requests/sec** | 0.50 req/sec | **60.46 req/sec** | **120x Faster** |
| **Success Rate** | 0% (100% timeouts) | **100% (1,817 reqs)** | **Fixed** |
| **Cause / Fix** | Synchronous DB write lock on GET | Moved `UPDATE view_count` to async `tokio::spawn` task & enabled SQLite WAL mode | ✅ Resolved |

---

### 4. Stress Test — 1,000 Concurrent Connections
```bash
wrk -t4 -c1000 -d30s --latency http://127.0.0.1:3000/
```

| Metric | Value |
|--------|-------|
| **Requests/sec** | **12,042.80** |
| **Transfer Rate** | **1.45 MB/sec** |
| **p50 Latency** | **110.29 ms** |
| **p90 Latency** | **145.89 ms** |
| **p99 Latency** | **247.81 ms** |
| Total Requests | **362,427** |
| Crashes / Downtime | **0** |

---

## 🛠️ Optimizations Applied to ImgHost

1. **SQLite Write-Ahead Logging (WAL)**:
   ```rust
   sqlx::query("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA cache_size=10000;")
       .execute(&pool)
       .await?;
   ```
   WAL mode allows unlimited concurrent readers while a write operation is being executed.

2. **Async Fire-and-Forget View Count Increments**:
   Instead of blocking the HTTP response on an SQL `UPDATE`, view count increments are offloaded to background Tokio tasks:
   ```rust
   tokio::spawn({
       let pool = pool.clone();
       let id = id.clone();
       async move {
           let _ = sqlx::query("UPDATE images SET view_count = view_count + 1 WHERE id = ?")
               .bind(&id)
               .execute(&pool)
               .await;
       }
   });
   ```

3. **Connection Pool Expansion**:
   Increased SQLite pool size from 5 to 20 connections to handle spike concurrency gracefully.
