<div align="center">

# ⚡ Process Manager

</div>

<img width="907" height="516" alt="Process Manager" src="https://github.com/user-attachments/assets/e90e2902-3f71-471f-b5b4-6dd01e3a7d94" />

Manage, monitor, and control long-running commands (dev servers, tunnels, background jobs) from a single interface. Define processes once, start/stop them with a click, view live logs with ANSI colors, and track resource usage. Think PM2 or systemd, but with a native GUI.

## Это форк [ByteTrix/process-manager](https://github.com/ByteTrix/process-manager)

Ключевые изменения:

- Увеличил размер шрифта и изменил цветовую схему, сделав ее более контрастной.
- Автоматическое сохранение настроек.
- При сворачивании в трей приложение занимает гораздо меньше оперативной памяти.
- Удалил встроенный терминал.
- Добавил опцию `–start-in-tray` — приложение запустится свернутым в трей.
- Добавил ротацию логов.
- CPU и Memory учитывают дочерние процессы.
- Слегка поменял интерфейс.
- Исправил баги ("Stop" теперь закрывает дочерние процессы, "auto-start" запускает процессы при старте и т.д.).

## Tech Stack

- Tauri 2 (Rust)
- Vue 3 + TypeScript + Pinia

## Build

```bash
bun install
bun run tauri build
```
