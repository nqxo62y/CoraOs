# CoraOS - Server Management Platform

A production-grade Debian Linux server management platform built with Rust and a custom web dashboard.

## Architecture

```
CoraOS/
├── backend/          # Rust API server (axum + tokio + sqlx)
│   └── src/
│       ├── config/       # App configuration, database, logging
│       ├── middleware/   # JWT auth extraction, role-based access
│       ├── models/       # Data types (user, auth, system, backup, audit)
│       ├── routes/       # HTTP route handlers
│       └── services/     # Business logic (auth, monitor, backup, etc.)
├── frontend/         # Web dashboard (HTML/CSS/JS)
│   └── dist/
│       ├── css/          # Professional dark/light theme styles
│       └── js/           # API client and application logic
├── config/           # Deployment configuration
│   ├── coraos.service    # systemd unit file
│   └── install.sh        # Automated installation script
├── cora-welcome/     # Console welcome utility (original)
├── scripts/          # ISO build scripts
└── .github/workflows/# CI/CD pipeline
```

## Features

- **Authentication** - JWT-based login with Argon2 password hashing
- **Role-Based Access Control** - Admin, Operator, Viewer roles
- **Real-Time Monitoring** - CPU, RAM, disk, network via WebSocket
- **Process Monitor** - Live process list sorted by resource usage
- **Service Management** - Start/stop/restart systemd services
- **Log Viewer** - Query journalctl with filters and search
- **Update Checker** - Check and apply apt package updates
- **Backup System** - Create/manage compressed tar backups
- **Configuration Editor** - Key-value server settings
- **Audit Logging** - Track all user actions
- **Dark/Light Themes** - Professional responsive UI

## Requirements

- Debian 12 (Bookworm) or later
- Rust 1.70+ (for building)
- SQLite 3.x (bundled via sqlx)

## Quick Start

### Build

```bash
cd backend
cargo build --release
```

### Run (Development)

```bash
cp .env.example .env
# Edit .env with your settings
cd backend
cargo run
```

### Install (Production)

```bash
sudo bash config/install.sh
```

The server will be available at `http://<your-lan-ip>:8080`

### Default Credentials

- Username: `admin`
- Password: `admin123`

**Change the default password immediately after first login.**

## API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/auth/login | No | Authenticate user |
| GET | /api/auth/me | Yes | Current user info |
| POST | /api/auth/logout | Yes | End session |
| GET | /api/system/metrics | Yes | System metrics |
| GET | /api/system/processes | Yes | Process list |
| GET | /api/services | Yes | List services |
| POST | /api/services/:name/action | Operator+ | Service action |
| GET | /api/logs | Yes | Query system logs |
| GET | /api/updates/check | Yes | Check for updates |
| POST | /api/updates/apply | Admin | Apply updates |
| GET | /api/backups | Yes | List backups |
| POST | /api/backups | Operator+ | Create backup |
| DELETE | /api/backups/:id | Admin | Delete backup |
| GET | /api/users | Admin | List users |
| POST | /api/users | Admin | Create user |
| PUT | /api/users/:id | Admin | Update user |
| DELETE | /api/users/:id | Admin | Delete user |
| GET | /api/config | Yes | List config |
| POST | /api/config | Admin | Update config |
| GET | /api/audit | Yes | Audit logs |
| GET | /api/ws | Yes | WebSocket metrics |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| CORAOS_HOST | 0.0.0.0 | Bind address |
| CORAOS_PORT | 8080 | Bind port |
| DATABASE_URL | sqlite:../data/coraos.db?mode=rwc | SQLite path |
| JWT_SECRET | (random) | Token signing key |
| JWT_EXPIRATION_HOURS | 24 | Token lifetime |
| BACKUP_PATH | ../backups | Backup storage |
| LOG_PATH | ../logs | Log storage |
| RUST_LOG | info | Log level |

## Security

- Passwords hashed with Argon2id
- JWT tokens with configurable expiration
- Input validation on all endpoints
- Service name sanitization (prevents command injection)
- Path traversal protection in backup system
- Role-based access control on all sensitive operations
- Audit logging of all administrative actions
- systemd hardening (NoNewPrivileges, ProtectSystem, etc.)

## License

MIT
