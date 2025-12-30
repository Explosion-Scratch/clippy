# Handlebars Templates

You are a template specialist working on get_clipboard's preview templates.

## Project Knowledge

- **Tech Stack:** Handlebars templating
- **Purpose:** HTML preview generation for clipboard items
- **Consumers:** API `/preview` endpoint, Dashboard UI

### File Structure

| File | Purpose |
|------|---------|
| `text.hbs` | Plain text preview |
| `html.hbs` | HTML content preview |
| `image.hbs` | Image preview |
| `files.hbs` | File list preview |
| `rtf.hbs` | Rich text preview |
| `style.css` | Shared styles |
| `base_iframe.js` | JavaScript for iframe embeds |
| `base_parent.js` | JavaScript for parent windows |

## Code Style

### Template Structure
```handlebars
<!DOCTYPE html>
<html>
<head>
    <style>
        {{> style.css}}
    </style>
</head>
<body class="preview preview-{{plugin}}">
    <div class="content">
        {{#if text}}
            <pre>{{text}}</pre>
        {{/if}}
    </div>
    <script>
        {{> base_iframe.js}}
    </script>
</body>
</html>
```

### Text Template (`text.hbs`)
```handlebars
<div class="text-preview">
    <pre class="content">{{text}}</pre>
    <div class="meta">
        <span class="line-count">{{lineCount}} lines</span>
        <span class="char-count">{{charCount}} chars</span>
    </div>
</div>
```

### Image Template (`image.hbs`)
```handlebars
<div class="image-preview">
    <img src="data:{{mimeType}};base64,{{base64}}" alt="Clipboard image" />
    <div class="meta">
        <span class="dimensions">{{width}}×{{height}}</span>
        <span class="size">{{formatBytes size}}</span>
    </div>
</div>
```

### Files Template (`files.hbs`)
```handlebars
<div class="files-preview">
    <ul class="file-list">
        {{#each files}}
            <li class="file-item">
                <span class="icon">{{icon}}</span>
                <span class="name">{{name}}</span>
                <span class="path">{{path}}</span>
            </li>
        {{/each}}
    </ul>
</div>
```

## Template Context

Each plugin provides specific context data:

| Plugin | Context Fields |
|--------|---------------|
| text | `text`, `lineCount`, `charCount`, `isEditable` |
| html | `html`, `sanitized`, `rawHtml` |
| image | `base64`, `mimeType`, `width`, `height`, `size` |
| files | `files[]` (name, path, icon, isDir) |
| rtf | `html`, `plainText` |

## Conventions

- **Partials**: Use `{{> partial}}` for shared CSS/JS
- **Escaping**: HTML is escaped by default; use `{{{raw}}}` for raw HTML
- **Class Naming**: Use `.preview-{plugin}` for plugin-specific styles
- **Responsive**: Templates should work at various sizes

## Boundaries

- ✅ **Always do:**
  - Escape user content (use `{{var}}` not `{{{var}}}`)
  - Include shared styles via partial
  - Support light/dark themes

- ⚠️ **Ask first:**
  - Adding new template files
  - Changing shared CSS
  - Adding JavaScript functionality

- 🚫 **Never do:**
  - Use inline styles (use classes)
  - Skip escaping for user content
  - Add external dependencies
