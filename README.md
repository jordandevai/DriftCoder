# DriftCode

**Your code, wherever you drift. No server install. Just SSH.**

DriftCode is a lightweight, cross-platform code editor that lets you securely edit code on your own machines over standard SSH. No server components, no cloud lock-in — just your environment, remotely.

Created by Jordan Gonzales (Jtech Minds LLC)  
https://jtechminds.com • https://driftcoder.com

---

## Features

- **Zero Server Footprint** — SSH only
- **Cross-Platform** — Linux, Windows, macOS, Android, iOS
- **Lightweight** — ~15MB bundle
- **Full Code Editor** — Syntax highlighting, search, multi-file editing
- **Integrated Terminal** — Full PTY access
- **Flexible Layout** — Draggable panels and tabs

---

## Licensing Overview (Important)

DriftCode is **source-available**, not open-source.

### Desktop (Windows / macOS / Linux)
- **Free** for personal and other non-commercial use
- **Commercial use** requires a paid license  
  → See `COMMERCIAL_LICENSE.md` ($15/seat/month)

### Android
- **Freemium** app via Google Play
- Core features available for free
- **Premium features unlocked via subscription**
- Billing and entitlement handled through Google Play

> Redistribution of modified binaries or removal of licensing / billing mechanisms is not permitted.

---

## Development

### Prerequisites
- Node.js 18+
- Rust 1.77+
- Platform-specific Tauri dependencies  
  https://v2.tauri.app/start/prerequisites/

### Setup

```bash
npm install
npm run tauri:dev
````

### Build

```bash
npm run tauri:build
```

---

## Android Builds (Developers)

This repository is public to allow:

* source inspection
* learning
* contributions
* non-commercial experimentation

Local Android builds are permitted **for personal, non-commercial use only**.

Publishing modified APKs, removing billing logic, or redistributing binaries outside approved app stores is **not permitted**.

---

## Project Structure

```
driftcode/
├── src/            # Svelte frontend
├── src-tauri/      # Rust backend
├── LICENSE.md
├── COMMERCIAL_LICENSE.md
└── README.md
```

---

## License

* Personal & non-commercial use: `LICENSE.md`
* Commercial use: `COMMERCIAL_LICENSE.md`
