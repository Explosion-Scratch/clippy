#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/../src/data"
OUTPUT_FILE="$OUTPUT_DIR/demo-items.json"
PREVIEWS_FILE="$OUTPUT_DIR/demo-previews.json"

API_URL="${CLIPBOARD_API_URL:-http://localhost:3016}"
ITEM_COUNT="${ITEM_COUNT:-8}"

mkdir -p "$OUTPUT_DIR"


if [ "$#" -gt 0 ]; then
    # Join arguments with comma
    IDS=$(IFS=,; echo "$*")
    echo "Fetching specific items: $IDS"
    items=$(curl -sf "$API_URL/items?ids=$IDS" || echo "[]")
else
    echo "Fetching $ITEM_COUNT recent items..."
    items=$(curl -sf "$API_URL/items?count=$ITEM_COUNT" || echo "[]")
fi

if [ -z "$items" ] || [ "$items" = "[]" ]; then
    echo "No items found or API not running at $API_URL"
    echo "Make sure to run: get_clipboard api"
    exit 1
fi

item_count=$(echo "$items" | jq 'length')
echo "Fetched $item_count items, processing..."

full_items="["
previews="{"
first=true

for i in $(seq 0 $((item_count - 1))); do
    item=$(echo "$items" | jq ".[$i]")
    id=$(echo "$item" | jq -r '.id')
    index=$(echo "$item" | jq -r '.index')
    
    echo "  Processing item $index: $id"
    
    full_data=$(curl -sf "$API_URL/item/$id/data" || echo "{}")
    
    if [ "$full_data" = "{}" ]; then
        echo "    Warning: Could not fetch full data for $id"
        continue
    fi
    
    preview_json=$(curl -sf "$API_URL/item/$id/preview?interactive=false" || echo "{}")
    
    if [ "$first" = true ]; then
        first=false
    else
        full_items+=","
        previews+=","
    fi
    
    full_items+="$full_data"
    previews+="\"$id\":$preview_json"
done

full_items+="]"
previews+="}"

echo "$full_items" | jq '.' > "$OUTPUT_FILE"
echo "$previews" | jq '.' > "$PREVIEWS_FILE"

echo ""
echo "Done! Generated:"
echo "  - $OUTPUT_FILE (item data)"
echo "  - $PREVIEWS_FILE (preview JSON)"
echo ""
echo "The demo data has been updated from your real clipboard history."
