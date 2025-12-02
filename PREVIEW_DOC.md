# Preview System Documentation

The Clippy preview system allows for generating and displaying previews of clipboard items. It is designed to be flexible, supporting various content types and display modes.

## API Endpoint

The primary endpoint for retrieving previews is:

`GET /item/:id/preview`

### Query Parameters

-   `interactive` (boolean, default: `true`): Controls whether the preview should include interactive elements (e.g., zoom controls for images, copy buttons for files). Set to `false` for static previews.

### Response Format

The endpoint returns a JSON object containing the preview data.

```json
{
  "id": "item_id",
  "formatsOrder": ["image", "html", "text"],
  "data": {
    "image": {
      "html": "<iframe src='...'></iframe>",
      "width": 800,
      "height": 600
    },
    "html": {
      "html": "..."
    },
    "text": {
      "html": "..."
    }
  }
}
```

## Templates

Previews are generated using Handlebars templates located in `get_clipboard/templates`.

-   `image.html`: For image previews. Supports zoom and pan.
-   `file.html`: For file lists.
-   `text.html`: For text content.
-   `html.html`: For HTML content.

### Styling

Styles are defined in `style.css`. The system supports:
-   **Transparent Backgrounds**: The root `body` and `html` are transparent to allow for seamless integration with overlay windows.
-   **Content Wrapper**: A `.content-wrapper` class provides a white background and rounded corners for the actual content.
-   **Dark Mode**: Supported via `@media (prefers-color-scheme: dark)`.

## Usage in Frontend

To display a preview:

1.  Fetch the preview data from the API.
2.  Select the preferred format from `formatsOrder`.
3.  Render the `html` content from the selected format in an `iframe`.
4.  For static previews (e.g., in a side window), append `?interactive=false` to the iframe source URL if applicable (though currently the API returns the full HTML, so the interactive flag is handled within the template logic via URL parameters passed to the template or handled by the template script if it parses the parent URL).

*Note: The current implementation passes the `interactive` flag to the template via the URL query parameter of the page serving the template, or the template logic itself adapts based on context.*
