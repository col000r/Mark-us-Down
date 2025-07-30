# Mark-us-Down

A fast, native **Markdown Editor** with live preview. Built with Tauri and React.

➡️ [Mark-us-Down Website](https://mark-us-down.brightlight.rocks)

![Mark-us-Down Logo](ME_Logo192.png)

## Features

- **Split-pane editor** with live preview and synchronized scrolling
- **Syntax highlighting** powered by Monaco Editor  
- **Native performance** - lightweight Tauri app, not Electron
- **Cross-platform** - macOS, Windows, and Linux
- **Dark/Light themes** with keyboard shortcut (`Cmd/Ctrl+T`)
- **Drag & drop** markdown files to open

## Installation

### Download
Get the latest release from the [Releases](https://github.com/col000r/Mark-us-Down/releases) page.

### Build from Source

```bash
# Prerequisites: Node.js 18+, Rust, and platform build tools

git clone https://github.com/col000r/Mark-us-Down.git
cd Mark-us-Down
npm install
npm run tauri build
```

## Usage

| Action | Shortcut |
|--------|----------|
| New File | `Cmd/Ctrl+N` |
| Open | `Cmd/Ctrl+O` |
| Save | `Cmd/Ctrl+S` |
| Save As | `Cmd/Ctrl+Shift+S` |
| Toggle Theme | `Cmd/Ctrl+T` |

## Development

```bash
npm run dev    # Run in development mode
npm run build  # Build for production
```

## Tech Stack

- **Frontend**: React 18, TypeScript, Monaco Editor
- **Backend**: Tauri 2.1, Rust
- **Build**: Vite

## License

Copyright (C) 2025 [Bright Light Interstellar Ltd.](https://brightlight.rocks)  
Blog: https://markus.hofer.rocks

[MIT License](LICENSE)