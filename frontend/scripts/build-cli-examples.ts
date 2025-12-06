import { spawn } from "bun";
import {readFileSync} from "fs";
import { resolve } from "path";

const OUTPUT_PATH = "src/data/cli-examples.json";

// CLI command to run (assumed to be in PATH)
const CLI = resolve(`${import.meta.dir}/../../get_clipboard/target/release/get_clipboard`);

// Mock responses for commands that shouldn't be run or are interactive
const MOCKS: Record<string, string> = {
  [`${CLI} interactive`]: readFileSync(`${import.meta.dir}/interactive-output.txt`, 'utf-8'),

  [`${CLI} copy 0`]: "Copied to clipboard: async function fetchUserData(userId) { ... }",
  [`${CLI} paste 0`]: "Copied and pasted: async function fetchUserData(userId) { ... }",
  [`${CLI} delete 0`]: "Deleted 1 item from history",

  [`${CLI} dir set /path/to/new/dir`]: "Data directory set to: /path/to/new/directory",
  [`${CLI} dir move /path/to/new/dir`]: "Moving data directory...\nMoved 847 items to: /path/to/new/directory\nData directory updated successfully",

  [`${CLI} service start`]: "Starting clipboard service...\nService started successfully (PID: 12345)",
  [`${CLI} service stop`]: "Stopping clipboard service...\nService stopped",
  [`${CLI} service install`]: "Installing launchd service...\nCreated: ~/Library/LaunchAgents/com.clippy.get_clipboard.plist\nService installed successfully",
  [`${CLI} service uninstall`]: "Uninstalling launchd service...\nRemoved: ~/Library/LaunchAgents/com.clippy.get_clipboard.plist\nService uninstalled",
  // logs might hang, so mock it
  [`${CLI} service logs`]: "=== Clipboard Change Detected ===\nAll formats (8 total):\n  • public.html: \"<meta charset='utf-8'><div data-v-8ac1771b=\"\" class=\"command-header\" style=\"box-sizing: border-box; margin: 0px; padd...\"\n  • Apple HTML pasteboard type: \"<meta charset='utf-8'><div data-v-8ac1771b=\"\" class=\"command-header\" style=\"box-sizing: border-box; margin: 0px; padd...\"\n  • public.utf8-plain-text: \"get_clipboard history --query --from --to --sort --json --text --full --image --file --limit --help\"\n  • NSStringPboardType: \"get_clipboard history --query --from --to --sort --json --text --full --image --file --limit --help\"\n  • dyn.ah62d4rv4gu8y63n2nuuhg5pbsm4ca6dbsr4gnkduqf31k3pcr7u1e3basv61a3k: <empty>\n  • NeXT smart paste pasteboard type: <empty>\n  • org.chromium.internal.source-rfh-token: <binary data, 24 bytes>\n  • org.chromium.source-url: \"http://localhost:5173/#demo\"\n================================\n\nStored clipboard item: get_clipboard history --query --from --to --sort --json --text --full --image --file --limit --help [1 copies]\n\n=== Clipboard Change Detected ===\nAll formats (7 total):\n  • public.html: \"<meta charset='utf-8'><div style=\"color: #a9b1d6;background-color: #1a1b26;font-family: Menlo, Monaco, 'Courier New',...\"\n  • Apple HTML pasteboard type: \"<meta charset='utf-8'><div style=\"color: #a9b1d6;background-color: #1a1b26;font-family: Menlo, Monaco, 'Courier New',...\"\n  • public.utf8-plain-text: \"frontend/src/data\"\n  • NSStringPboardType: \"frontend/src/data\"\n  • org.chromium.internal.source-rfh-token: <binary data, 24 bytes>\n  • org.chromium.web-custom-data: <binary data, 776 bytes>\n  • org.chromium.source-url: \"vscode-file://vscode-app/Applications/Antigravity.app/Contents/Resources/app/out/vs/code/electron-browser/workbench/w...\"\n================================\n\nStored clipboard item: frontend/src/data [1 copies]\n\n=== Clipboard Change Detected ===\nAll formats (4 total):\n  • public.utf8-plain-text: \"bun run scripts/build-cli-examples.ts\"\n  • NSStringPboardType: \"bun run scripts/build-cli-examples.ts\"\n  • org.chromium.internal.source-rfh-token: <binary data, 24 bytes>\n  • org.chromium.source-url: \"vscode-file://vscode-app/Applications/Antigravity.app/Contents/Resources/app/out/vs/code/electron-browser/workbench/w...\"\n================================\n\nStored clipboard item: bun run scripts/build-cli-examples.ts [1 copies]",

  [`${CLI} permissions request`]: "Opening System Settings to request accessibility permissions...",


  [`${CLI} export backup.json`]: "Exporting 847 items to backup.json...\n  Processed 100/847 items\n  Processed 200/847 items\n  ...\nExported 847 items successfully",
  [`${CLI} import backup.json`]: "Importing from backup.json (847 items)...\n  [1/847] Imported: async function fetchUserData...\n  [2/847] Imported: https://github.com/example\n  ...\nImport complete: 324 imported, 523 skipped (already exist)",
};

// Configuration of commands and their scenarios
// The key is the subcommand (e.g., 'history')
// The value is an object mapping user-facing keys (e.g., '--limit') to the command string to run
const CONFIG = {
  history: {
    default: `${CLI} history --limit 10`,
    "--limit": `${CLI} history --limit 3`,
    "--json": `${CLI} history --json --limit 3`,
    "--text": `${CLI} history --text --limit 5`,
    "--image": `${CLI} history --image --limit 5`,
    "--html": `${CLI} history --html --limit 5`,
    "--file": `${CLI} history --file --limit 5`,
    "--help": `${CLI} history --help`,
    // "--query": `${CLI} history --query "test" --limit 5`, // Optional, maybe better in search
  },
  show: {
    default: `${CLI} show 0`,
    "--json": `${CLI} show --json 0`,
    "--text": `${CLI} show --text 0`,
    "--image": `${CLI} show --image 0`,
    "--file": `${CLI} show --file 0`,
    "--html": `${CLI} show --html 0`,
    "--help": `${CLI} show --help`,
  },
  search: {
    default: `${CLI} search function --limit 5`,
    "function": `${CLI} search function --limit 5`,
    "--limit": `${CLI} search function --limit 2`,
    "--json": `${CLI} search http --json --limit 2`,
    "--regex": `${CLI} search ^https --regex --limit 5`,
    "--sort": `${CLI} search http --sort relevance --limit 5`,
    "--help": `${CLI} search --help`,
  },
  dir: {
    get: `${CLI} dir get`,
    set: `${CLI} dir set /path/to/new/dir`,
    move: `${CLI} dir move /path/to/new/dir`,
    "--help": `${CLI} dir --help`,
  },
  service: {
    status: `${CLI} service status`,
    start: `${CLI} service start`,
    stop: `${CLI} service stop`,
    install: `${CLI} service install`,
    uninstall: `${CLI} service uninstall`,
    logs: `${CLI} service logs`,
    "--help": `${CLI} service --help`,
  },
  copy: {
    default: `${CLI} copy 0`,
    "--help": `${CLI} copy --help`,
  },
  paste: {
    default: `${CLI} paste 0`,
    "--help": `${CLI} paste --help`,
  },
  delete: {
    default: `${CLI} delete 0`,
    "--help": `${CLI} delete --help`,
  },
  interactive: {
    default: `${CLI} interactive`,
    "--help": `${CLI} interactive --help`,
  },
  permissions: {
    check: `${CLI} permissions check`,
    request: `${CLI} permissions request`,
    "--help": `${CLI} permissions --help`,
  },
  // Include stats/export/import if they exist or to show they don't
  stats: {
      default: `${CLI} stats`,
      "--json": `${CLI} stats --json`,
      "--help": `${CLI} stats --help`,
  },
  export: {
      default: `${CLI} export backup.json`,
      "--help": `${CLI} export --help`,
  },
  import: {
      default: `${CLI} import backup.json`,
      "--help": `${CLI} import --help`,
  }
};

async function runCommand(cmd: string): Promise<string> {
  // Check mocks first
  if (MOCKS[cmd]) {
    return MOCKS[cmd];
  }

  try {
    const proc = spawn(cmd.split(" "), {
        stdout: "pipe",
        stderr: "pipe",
    });

    const output = await new Response(proc.stdout).text();
    const error = await new Response(proc.stderr).text();
    await proc.exited;

    if (error && !output) {
        // If there's only error output, return it (e.g. for --help or errors)
        return error.trim();
    }
    
    // Combining output and error can be useful if help prints to stderr or something
    return (output + error).trim();

  } catch (e) {
    console.warn(`Failed to run command "${cmd}":`, e);
    return `Error running command: ${e}`;
  }
}

async function main() {
  const helpOnly = process.argv.includes("--help-only");
  
  if (helpOnly) {
    console.log("Building CLI examples (--help only)...");
  } else {
    console.log("Building CLI examples...");
  }
  
  let results: Record<string, Record<string, [string, string]>> = {};
  
  if (helpOnly) {
    try {
      const existingData = await Bun.file(OUTPUT_PATH).text();
      results = JSON.parse(existingData);
      console.log("Loaded existing data from", OUTPUT_PATH);
    } catch (e) {
      console.warn("Could not read existing data file, starting fresh");
    }
  }

  for (const [subcommand, variations] of Object.entries(CONFIG)) {
    console.log(`Processing subcommand: ${subcommand}`);
    if (!results[subcommand]) {
      results[subcommand] = {};
    }
    
    for (const [key, cmdString] of Object.entries(variations)) {
      if (helpOnly && key !== "--help") {
        continue;
      }
      
      process.stdout.write(`  - ${key}... `);
      const output = await runCommand(cmdString);
      const displayCmd = cmdString.replace(CLI, "get_clipboard");
      const displayOutput = output.replace(new RegExp(CLI.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), "get_clipboard");
      results[subcommand][key] = [displayCmd, displayOutput];
      console.log("Done");
    }
  }

  await Bun.write(OUTPUT_PATH, JSON.stringify(results, null, 2));
  console.log(`\nWrote examples to ${OUTPUT_PATH}`);
}

main().catch(console.error);
