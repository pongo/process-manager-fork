<div align="center">

# ⚡ Process Manager

</div>

![Home](images/home.png)

Manage, monitor, and control long-running commands (dev servers, tunnels, background jobs) from a single interface. Define processes once, start/stop them with a click, view live logs with ANSI colors, and track resource usage. Think PM2 or systemd, but with a native GUI.

## 🎯 Why?

Stop juggling multiple terminal windows or remembering `pm2` commands. Define your dev servers, background workers, or tunnels once, then start/stop/monitor them from a clean UI. Perfect for developers running local microservices, Cloudflare tunnels, database instances, or any long-running CLI tools.

## ✨ Highlights

- ⚡ Process list with quick start/stop/restart actions
- 📡 Real-time stdout/stderr streaming with ANSI color rendering
- 📊 CPU / memory usage panel with uptime + PID metadata
- 🔁 Auto-restart and auto-start toggles per process
- 💾 Config save/load persistence
- 🗂️ Minimize-to-tray workflow with tray menu actions

## 📸 Screenshots

**Task page - logs, metrics & controls**
![Task page](images/task%20page.png)

## 🛠️ Tech Stack

- ⚡ **Desktop shell:** Tauri 2 (Rust)
- 🎨 **Frontend:** Vue 3 + TypeScript + Pinia
- 📈 **Monitoring:** `sysinfo`
- 🪟 **Windows integration:** tray icon/menu

## 📋 Requirements

- 🪟 Windows 10/11
- 🟢 Node.js 18+
- 🦀 Rust toolchain (stable)
- 🌐 WebView2 Runtime (usually already present on modern Windows)

## 🚀 Quick Start

Install dependencies:

```bash
bun install
```

Run desktop app in development:

```bash
bun run tauri dev
```

Frontend-only dev server:

```bash
bun run dev
```

Build production web assets:

```bash
bun run build
```

Build production desktop app:

```bash
bun run tauri build
```

Launch the installed executable minimized to the system tray:

```text
ProcessManager.exe --start-in-tray
```

## 📖 Using the App

### ⚡ Manage processes

1. Click **New Process** in the sidebar.
2. Enter a name + command (+ optional working directory).
3. Use row actions to start/stop/restart.
4. Select a process to inspect logs and metrics in the right panel.

## 🗃️ Data & Logs

- 💾 Process configs and logs are stored under your app data directory.
- 📜 Logs are persisted per-process and streamed live to the UI.

## 🗂️ Tray Behavior

- 🔽 Closing the window hides the app to tray.
- 📋 Tray menu supports show/hide/start all/stop all/quit.
- 🖱️ Left-click tray icon brings the main window back.

## 📜 Scripts

From `package.json`:

- `bun run dev` → ⚡ Vite dev server
- `bun run build` → 🔨 Type-check + production web build
- `bun run preview` → 👁️ Preview built web assets
- `bun run tauri dev` → 🚀 Run Tauri desktop app in dev mode
- `bun run tauri build` → 📦 Build release desktop bundle

## 🔧 Troubleshooting

### ⚡ Process command not found

- Verify the command exists in your PowerShell PATH:
  - `Get-Command <tool-name>`
- Restart the app after installing a new CLI tool.

### 📭 No logs in panel

- Confirm process status is `Running`.
- Check the command actually writes to stdout/stderr.

### 🗂️ Tray menu not appearing

- Right-click the tray icon for the context menu.
- Left-click is reserved to restore/focus the main window.

## 📄 License

MIT
