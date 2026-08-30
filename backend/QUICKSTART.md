# Quick Start Guide

## Option 1: Run with Docker/Podman (Easiest)

```bash
# Start PostgreSQL in container
podman run -d \
  --name clovalink-db \
  -e POSTGRES_USER=clovalink \
  -e POSTGRES_PASSWORD=clovalink \
  -e POSTGRES_DB=clovalink \
  -p 5432:5432 \
  postgres:16-alpine

# Run migrations
psql -h localhost -U clovalink -d clovalink < migrations/001_initial_schema.sql
psql -h localhost -U clovalink -d clovalink < migrations/002_seed_demo_data.sql

# Run backend
cd backend
cargo run
```

## Option 2: Local PostgreSQL

```bash
# Create database
createdb clovalink

# Create user (if needed)
createuser -P clovalink  # password: clovalink

# Run migrations
psql -d clovalink < migrations/001_initial_schema.sql
psql -d clovalink < migrations/002_seed_demo_data.sql

# Update .env with your database URL
# DATABASE_URL=postgresql://username:password@localhost:5432/clovalink

# Run backend
cargo run
```

## Option 3: Compile without Database

If you just want to get the frontend working first:

```bash
# Comment out all sqlx::query! macros temporarily
# Or use: cargo build --features offline

# The backend won't work but frontend can be developed
```

## Test Backend

```bash
# Login
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@acme.com","password":"password123"}'

# Create file request (use token from login)
curl -X POST http://localhost:3000/api/file-requests \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Test","destination_path":"/test","expires_in_days":7}'
```
