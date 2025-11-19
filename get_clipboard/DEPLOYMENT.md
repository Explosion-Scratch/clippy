# Clippith Deployment Guide

## Quick Start

1. **Build the frontend:**
   ```bash
   ./build-frontend.sh
   ```

2. **Run the server:**
   ```bash
   cargo run -- serve --port 3000
   ```

3. **Access the dashboard:**
   Open http://127.0.0.1:3000/dashboard/ in your browser

## Architecture

```
┌─────────────────────────────────────────┐
│         Rust Backend (Axum)             │
│  ┌───────────────────────────────────┐  │
│  │  Static Files: /dashboard/        │  │
│  │  (frontend-dist/)                 │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  API Endpoints:                   │  │
│  │  - /items                         │  │
│  │  - /search                        │  │
│  │  - /stats                         │  │
│  │  - etc.                           │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────┐
│    Vue.js Dashboard (Static Build)      │
│  - Built to: frontend-dist/             │
│  - Served at: /dashboard/               │
│  - Base path: /dashboard/               │
└─────────────────────────────────────────┘
```

## Build Process

### Frontend Build

The `build-frontend.sh` script does the following:

1. Installs npm dependencies in `frontend-app/`
2. Runs Vite build with base path `/dashboard/`
3. Outputs static files to `frontend-dist/`

**Output structure:**
```
frontend-dist/
├── index.html           # Entry point with /dashboard/ paths
└── assets/
    ├── index-*.js       # Bundled JavaScript (~378KB, 125KB gzipped)
    └── index-*.css      # Bundled CSS (~27KB, 5.5KB gzipped)
```

### Backend Integration

The Rust backend uses `tower-http::ServeDir` to serve static files:

```rust
.nest_service("/dashboard", ServeDir::new(frontend_dist))
```

This serves all files from `frontend-dist/` at the `/dashboard/` URL path.

## Development Workflow

### Frontend Development

For development with Hot Module Replacement:

```bash
cd frontend-app
npm run dev
```

This starts the Vite dev server on http://localhost:5173 with API proxying to the backend.

### Backend Development

Run the backend separately:

```bash
cargo run -- serve --port 3000
```

### Full Stack Development

Terminal 1 (Backend):
```bash
cargo watch -x 'run -- serve --port 3000'
```

Terminal 2 (Frontend):
```bash
cd frontend-app && npm run dev
```

Access the app at http://localhost:5173 (Vite dev server with HMR)

## Production Build

1. Build the frontend:
   ```bash
   ./build-frontend.sh
   ```

2. Build the Rust binary:
   ```bash
   cargo build --release
   ```

3. Run the production server:
   ```bash
   ./target/release/get_clipboard serve --port 3000
   ```

The server will serve:
- API documentation at `/`
- Static dashboard at `/dashboard/`
- All API endpoints at their respective paths

## File Locations

- **Frontend source:** `frontend-app/src/`
- **Frontend build output:** `frontend-dist/`
- **Backend source:** `src/`
- **API documentation:** `API.md`
- **Build script:** `build-frontend.sh`

## Environment Variables

None required. The application uses:
- **Port:** Specified via `--port` flag (default: configurable)
- **Data directory:** Auto-detected or configurable via API

## URL Structure

- `http://127.0.0.1:3000/` - API documentation (plain text)
- `http://127.0.0.1:3000/dashboard/` - Vue.js dashboard (static)
- `http://127.0.0.1:3000/items` - API endpoint
- `http://127.0.0.1:3000/search` - API endpoint
- `http://127.0.0.1:3000/stats` - API endpoint
- etc.

## Static Build Verification

To verify the static build is correct:

1. Check paths in `frontend-dist/index.html` - should reference `/dashboard/assets/`
2. Check assets exist in `frontend-dist/assets/`
3. Run the server and access `/dashboard/` - should load without errors

## Troubleshooting

### Dashboard shows blank page

- Check browser console for errors
- Verify `frontend-dist/` exists and contains files
- Ensure paths in `index.html` start with `/dashboard/`

### API calls fail from dashboard

- Ensure backend is running
- Check browser network tab for CORS or connection errors
- Verify API endpoints are accessible

### Build fails

- Ensure Node.js and npm are installed
- Run `cd frontend-app && npm install` manually
- Check for npm errors

### Assets not loading

- Verify `vite.config.js` has `base: '/dashboard/'`
- Rebuild frontend after config changes
- Check that `ServeDir` points to correct directory

## Performance

The static build is optimized:
- **JavaScript:** ~378KB (125KB gzipped)
- **CSS:** ~27KB (5.5KB gzipped)
- **Images:** Inlined as data URLs
- **Fonts:** Loaded from Google Fonts CDN

First load is fast, subsequent loads use browser cache.

## Security

- **Local only:** Server binds to 127.0.0.1
- **No authentication:** Designed for local use
- **No HTTPS:** Local development only

For production deployment, consider:
- Reverse proxy with authentication
- HTTPS termination
- Rate limiting
- CORS configuration

## Updates

To update the dashboard:

1. Make changes in `frontend-app/src/`
2. Run `./build-frontend.sh`
3. Restart the backend server

The backend will automatically serve the new build.

