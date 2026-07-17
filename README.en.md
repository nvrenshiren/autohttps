<div align="center">

# autohttps

**Cross-platform HTTPS certificate lifecycle manager · desktop + server**

![Rust](https://img.shields.io/badge/Rust-1.9x-CE412B?logo=rust&logoColor=white)
![React](https://img.shields.io/badge/React-19-149ECA?logo=react&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)
![Tailwind CSS](https://img.shields.io/badge/Tailwind-v4-38BDF8?logo=tailwindcss&logoColor=white)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-555)

[中文](README.md) · **English**

Make "keep a list of domains and their certificates" effortless — detect on startup, auto-renew before expiry, and surface failures clearly.

</div>

---

## ✨ Features

- **Two issuance methods**
  - **ACME public CA** (Let's Encrypt, etc.): account registration, **HTTP-01 (webroot)** automatic validation, **DNS-01 (manual)** with a guided TXT-record wizard.
  - **Self-signed root CA**: create / import a root CA, issue trusted certificates for internal / local-dev use, and export the root CA for clients to trust.
- **Full certificate lifecycle**: issue → valid → expiry → renew / retry / revoke / delete / export (leaf / chain / private key, with a risk confirmation for private-key export).
- **Hands-off automation**: full scan on startup; policy-driven **auto-renewal** before expiry; crash-recovery on restart so tasks never get stuck.
- **Real-time visibility**: dashboard with three metrics (total / expiring-soon / failed) + a to-do list + a red dot; SSE pushes state changes so the UI refreshes instantly.
- **Two run modes, one frontend**
  - **Desktop** (Tauri): 800×600 window + system tray (stays resident, close-to-tray), autostart, tray red-dot badge.
  - **Server**: daemon + browser Web UI, 24/7.
- **Data safety**: private keys / ACME account keys / root-CA keys are encrypted at rest with [age](https://github.com/FiloSottile/age); the database stores only references, never plaintext, and logs are redacted.

---

## 🚀 Quick start

Prerequisites: Rust (1.9x), Node.js (20+).

```bash
# 1) Build the frontend (embedded into the binary)
cd frontend && npm install && npm run build && cd ..

# Server mode: browser Web UI
cargo run -p server          # → http://127.0.0.1:8443

# Desktop mode: 800×600 window + system tray
cargo run -p desktop
```

Env vars: `AUTOHTTPS_ADDR` (listen address, default `127.0.0.1:8443`) · `AUTOHTTPS_DATA_DIR` (data dir, default `./data`) · `AUTOHTTPS_ACME_CA_CERT` (testing: trust a custom ACME CA root).

To issue public certificates, point the ACME directory at a real CA, e.g. Let's Encrypt: `https://acme-v02.api.letsencrypt.org/directory`.

---

## 🏛 Architecture

**One Rust core + one React frontend**, shared by both run modes; only the "shell" differs. The frontend talks **only** over HTTP + SSE; the desktop mode embeds the same axum service in-process (loopback only), and both modes mount the **same** router — the contract is defined once.

```
crates/
  core/      domain core: state-machine enums (single source of truth) · SQLite (SeaORM)
             · ACME client · self-signed CA (rcgen) · age secret storage
             · task queue + executor · expiry scanner
  api/       transport: axum router (REST + global SSE /events) · DTOs · embedded SPA
  server/    server-mode daemon (bin)
  desktop/   desktop-mode Tauri v2 shell (bin, embeds api/core)
frontend/    React 19 + Vite + TypeScript, shared by both modes; talks to /api via react-query
docs/        prd (requirements) · architecture (DB / API contracts) · design (design system)
```

The **task queue is a SQLite table** (durable + crash-recoverable); **enums are single-sourced** in `core` and exported to the frontend; secrets are stored as `*_ref` with ciphertext on disk.

---

## 🧪 Testing ACME locally with Pebble

No real domain or public network needed — verify the full ACME client flow against Let's Encrypt's official test server [Pebble](https://github.com/letsencrypt/pebble):

```bash
git clone --depth 1 https://github.com/letsencrypt/pebble && cd pebble
go build -o pebble.exe ./cmd/pebble
PEBBLE_VA_ALWAYS_VALID=1 ./pebble.exe -config test/config/pebble-config.json   # → https://localhost:14000/dir

# Trust Pebble's CA root on the autohttps side
AUTOHTTPS_ACME_CA_CERT=<pebble>/test/certs/pebble.minica.pem cargo run -p server
```

In the UI, register an account (directory `https://localhost:14000/dir`) → issue an ACME certificate → HTTP-01 passes automatically / DNS-01 confirms via the wizard → the certificate becomes "valid".

---

## 🧱 Tech stack

| Layer | Choices |
| --- | --- |
| Backend | Rust · axum · SeaORM + SQLite (WAL) · tokio · [instant-acme](https://github.com/instant-labs/instant-acme) (ACME) · [rcgen](https://github.com/rustls/rcgen) (X.509 / self-signed) · [age](https://github.com/str4d/rage) (encryption) · rust-embed |
| Desktop | Tauri v2 (tray / single-instance / autostart plugins) |
| Frontend | React 19 · Vite · TypeScript · Tailwind CSS v4 · shadcn/ui + Radix · @tanstack/react-query · zustand · react-hook-form + zod · lucide-react · sonner |

---

## 📦 Status

The MVP is functionally complete (both the self-signed and ACME paths, both desktop and server modes); issue / renew / revoke / export, expiry scanning / auto-renewal, and real-time refresh are all verified end-to-end.

**Non-goals (out of scope for the MVP)**: auto-deploying certificates to nginx / apache (export only) · multi-channel notifications (red dot only) · Web UI authentication (local / trusted network only) · DNS-01 provider-API automation (manual only).

---

## 📄 License

[MIT](LICENSE)
