#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/../src/data/cli-examples.json"
BINARY="get_clipboard"

echo "Building CLI examples..."

run_cmd() {
    local output
    output=$($BINARY "$@" 2>&1 || true)
    echo "$output" | jq -Rs '.'
}

{
echo "{"
echo '  "history": {'
echo '    "default":' "$(run_cmd history --limit 10),"
echo '    "--limit_3":' "$(run_cmd history --limit 3),"
echo '    "--json":' "$(run_cmd history --json --limit 3),"
echo '    "--text":' "$(run_cmd history --text --limit 5),"
echo '    "--image":' "$(run_cmd history --image --limit 5),"
echo '    "--file":' "$(run_cmd history --file --limit 5)"
echo '  },'
echo '  "show": {'
echo '    "default":' "$(run_cmd show 0),"
echo '    "--json":' "$(run_cmd show --json 0)"
echo '  },'
echo '  "search": {'
echo '    "function":' "$(run_cmd search "function" --limit 5),"
echo '    "--regex":' "$(run_cmd search "^https" --regex --limit 5)"
echo '  },'
echo '  "service": {'
echo '    "status":' "$(run_cmd service status)"
echo '  },'
echo '  "dir": {'
echo '    "get":' "$(run_cmd dir get)"
echo '  },'
echo '  "stats": {'
echo '    "default":' "$(run_cmd stats),"
echo '    "--json":' "$(run_cmd stats --json)"
echo '  },'
echo '  "permissions": {'
echo '    "check":' "$(run_cmd permissions check)"
echo '  }'
echo "}"
} > "$OUTPUT_FILE"

echo "CLI examples built successfully!"
echo "Output: $OUTPUT_FILE"
