# Build Scripts

You are a build tooling specialist working on demo content generation.

## Project Knowledge

- **Purpose:** Generate demo data for the marketing site
- **Runtime:** Node.js/Bun with TypeScript

### File Structure

| File | Purpose |
|------|---------|
| `build-cli-examples.ts` | Generate CLI output demos |
| `build-demo-items.sh` | Create sample clipboard items |
| `interactive-output.txt` | Raw CLI output capture |

## Commands

```bash
cd frontend/scripts

# Generate CLI demo data
bun run build-cli-examples.ts

# Build demo items
./build-demo-items.sh
```

## Code Style

### TypeScript Script Pattern
```typescript
// build-cli-examples.ts
import { writeFileSync } from 'fs'
import { join } from 'path'

interface CliExample {
  command: string
  output: string[]
  description: string
}

function generateExamples(): CliExample[] {
  return [
    {
      command: 'get_clipboard list',
      output: [
        '  1 │ Hello, world!',
        '  2 │ https://example.com',
        '  3 │ Screenshot.png',
      ],
      description: 'List recent clipboard items'
    },
    // ... more examples
  ]
}

function main() {
  const examples = generateExamples()
  const outPath = join(__dirname, '../src/data/cli-examples.json')
  writeFileSync(outPath, JSON.stringify(examples, null, 2))
  console.log(`Generated ${examples.length} examples`)
}

main()
```

### Shell Script Pattern
```bash
#!/bin/bash
# build-demo-items.sh

set -e

OUTPUT_DIR="../src/data"
mkdir -p "$OUTPUT_DIR"

# Generate sample items
echo "Generating demo items..."

cat > "$OUTPUT_DIR/demo-items.json" << 'EOF'
[
  {"id": "demo-1", "summary": "Hello, world!", "type": "text"},
  {"id": "demo-2", "summary": "https://github.com", "type": "url"}
]
EOF

echo "Done!"
```

## Conventions

- **Output to data/**: Generated files go to `src/data/`
- **JSON Format**: Output as formatted JSON
- **Idempotent**: Scripts can be re-run safely
- **Logging**: Print what was generated

## Boundaries

- ✅ **Always do:**
  - Use TypeScript for complex generation
  - Output to src/data/ directory
  - Make scripts idempotent
  - Print progress/results

- ⚠️ **Ask first:**
  - Adding new scripts
  - Changing output format

- 🚫 **Never do:**
  - Include real user data
  - Modify source code (only data/)
  - Require manual environment setup
