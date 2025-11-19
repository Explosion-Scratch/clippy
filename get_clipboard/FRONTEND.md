# Clippith Frontend

This document describes the new Vue.js-based frontend for the Clippith dashboard.

## Architecture

The frontend has been completely rebuilt as a modern Vue 3 application with the following structure:

```
frontend-app/
├── src/
│   ├── components/         # Vue components
│   │   ├── Sidebar.vue      # Navigation and filters
│   │   ├── TopBar.vue       # Search bar
│   │   ├── ItemList.vue     # List of clipboard items
│   │   ├── ItemDetail.vue   # Detailed item view
│   │   ├── StatsModal.vue   # Statistics with charts
│   │   ├── ImportModal.vue  # Import functionality
│   │   ├── SettingsModal.vue # Settings dialog
│   │   └── ToastContainer.vue # Toast notifications
│   ├── composables/        # Vue composables
│   │   └── useClipboard.js  # Main state management
│   ├── App.vue             # Root component
│   ├── main.js             # Entry point
│   └── style.css           # Global styles
├── index.html              # HTML template
├── vite.config.js          # Vite configuration
├── tailwind.config.js      # Tailwind CSS config
└── package.json            # Dependencies

frontend-dist/              # Production build output
```

## Features

- ✨ **Component-Based**: Clean separation of concerns with reusable Vue components
- 🎨 **Notion-like UI**: Beautiful, minimal design with Tailwind CSS
- ⚡ **Fast**: Built with Vite for lightning-fast development and optimized builds
- 🎯 **Type-Safe**: All components use Vue 3 Composition API with proper prop types
- 📊 **Interactive Charts**: Chart.js integration for statistics visualization
- 🔍 **Real-time Search**: Debounced search with live results
- ⌨️ **Keyboard Navigation**: Full keyboard support (arrow keys, shortcuts)
- 🎭 **Icons**: Phosphor Icons for a consistent, modern look
- 📱 **Responsive**: Works on all screen sizes

## Development

### Setup

```bash
cd frontend-app
npm install
```

### Development Server

```bash
npm run dev
```

This starts the Vite dev server on http://localhost:5173 with HMR (Hot Module Replacement) enabled. API calls are proxied to `http://127.0.0.1:3000`.

### Building for Production

```bash
npm run build
```

This builds the app to `../frontend-dist/` which can be served by the Rust backend.

Alternatively, use the convenience script from the project root:

```bash
./build-frontend.sh
```

## Component Details

### Sidebar.vue
- Displays filters (All, Text, Images, Files)
- Shows action buttons (Import, Export, Stats, Settings)
- Item counts per filter
- Connection status indicator

### TopBar.vue
- Search input with keyboard shortcut (`/`)
- Refresh button
- Loading indicator

### ItemList.vue
- Virtualized scrolling for performance
- Multi-select with Cmd/Ctrl+Click and Shift+Click
- Compact single-line items
- Click index to copy
- Type icons

### ItemDetail.vue
- Format tabs for different clipboard data types
- Size display per format
- Image preview with checkerboard background
- HTML rendering
- Code syntax with copy button
- Metadata footer

### StatsModal.vue
- Key metrics cards (Total Items, Size, Types)
- Interactive Chart.js bar chart
- Click bars to see daily breakdowns
- Item ID chips that load items on click

### useClipboard.js
- Centralized state management
- API interaction layer
- Auto-refresh polling
- Toast notifications
- Keyboard event handling
- Multi-select logic

## Keyboard Shortcuts

- `/` - Focus search (from anywhere)
- `Esc` - Clear selection / Close modals
- `↑/↓` - Navigate items (planned)
- `Ctrl/Cmd + C` - Copy item (planned)
- `Delete` - Delete item(s) (planned)

## Styling

The app uses:
- **Tailwind CSS** for utility-first styling
- **Inter** font for UI text
- **JetBrains Mono** for code/monospace
- **Custom animations** for smooth transitions
- **Gradient accents** for visual hierarchy

Color palette follows Notion's minimalist approach:
- Grays for backgrounds and borders
- Blue for primary actions and selection
- Green/Red for success/error states
- Subtle shadows and hover effects

## API Integration

The frontend communicates with the Rust backend via REST API:

- `GET /items` - Fetch clipboard items (with pagination)
- `GET /item/:id` - Fetch single item
- `GET /item/:id/data` - Fetch full item data with formats
- `GET /search?query=...` - Search items
- `GET /stats` - Get statistics with history
- `GET /mtime` - Get last modification time
- `GET /dir` - Get data directory path
- `POST /dir` - Update data directory
- `POST /item/:id/copy` - Copy item to clipboard
- `DELETE /item/:id` - Delete item

## Future Enhancements

- [ ] Arrow key navigation between items
- [ ] Drag and drop file import
- [ ] Dark mode toggle
- [ ] Custom themes
- [ ] Bulk operations UI
- [ ] Advanced filtering
- [ ] Export format options
- [ ] Item tagging
- [ ] Search history
- [ ] Keyboard shortcuts panel

## Contributing

When adding new features:

1. Create components in `src/components/`
2. Add reusable logic to `src/composables/`
3. Update `useClipboard.js` for state management
4. Follow existing naming conventions
5. Use Tailwind CSS utilities
6. Ensure responsive design
7. Test keyboard navigation
8. Update this documentation

## License

Same as the parent project.

