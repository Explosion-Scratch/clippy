# Clippith Dashboard (Vue App)

A modern, feature-rich dashboard for the Clippith clipboard manager built with Vue 3, Vite, Tailwind CSS, and Phosphor Icons.

## Features

- 🎨 **Beautiful UI** - Notion-inspired design with smooth animations
- ⌨️ **Keyboard Navigation** - Full keyboard support (arrow keys, shortcuts)
- 🔍 **Advanced Search** - Real-time search with debouncing
- 📊 **Statistics** - Interactive charts showing clipboard usage
- 🎯 **Multi-select** - Select and manage multiple items
- 📁 **Type Filtering** - Filter by text, images, files
- 🌓 **Format Tabs** - View different formats of clipboard items
- ⚡ **Fast** - Built with Vite for lightning-fast HMR

## Development

```bash
# Install dependencies
npm install

# Start dev server (with API proxy to localhost:3000)
npm run dev

# Build for production
npm run build
```

## Keyboard Shortcuts

- `/` - Focus search
- `↑/↓` - Navigate items
- `Esc` - Clear selection / Close modals
- `Ctrl/Cmd + C` - Copy selected item
- `Delete` - Delete selected item(s)

## Architecture

### Components
- `Sidebar.vue` - Navigation and filters
- `TopBar.vue` - Search bar and actions
- `ItemList.vue` - Scrollable list of clipboard items
- `ItemDetail.vue` - Detailed view with format tabs
- `StatsModal.vue` - Statistics with interactive charts
- `ImportModal.vue` - Import clipboard data
- `SettingsModal.vue` - Configure data directory
- `ToastContainer.vue` - Toast notifications

### Composables
- `useClipboard.js` - Main state management and API calls

## API Proxy

The dev server proxies API calls to `http://127.0.0.1:3000`. Make sure the Clippith API server is running:

```bash
cargo run -- serve --port 3000
```

## Build

The production build outputs to `../frontend-dist` and can be served by the Rust backend.
