import { execSync } from 'child_process';

const WAIT_API = 1000;

async function benchmark() {
  console.log("Starting API server in background...");
  // Assuming the API is already running on port 3016
  
  // 1. Generate a random string to trace
  const testString = `benchmark-test-${Date.now()}`;
  
  console.log(`Copying unique string: ${testString}`);
  // 2. Put string into clipboard and have CLI save it
  const startCopy = performance.now();
  execSync(`printf "%s" "${testString}" | pbcopy`);
  const copyTime = performance.now() - startCopy;
  console.log(`pbcopy Time: ${copyTime.toFixed(2)}ms`);

  console.log("Waiting 1500ms for watch process to ingest clipboard...");
  await new Promise(r => setTimeout(r, 1500));

  console.log("Querying API for the string...");
  const startSearch = performance.now();
  
  const response = await fetch(`http://127.0.0.1:3016/search?query=${testString}`);
  const searchTime = performance.now() - startSearch;
  
  const json = await response.json();
  
  console.log(`API Search Time: ${searchTime.toFixed(2)}ms`);
  
  const hits = Array.isArray(json) ? json : json.hits;
  console.log("Hits found:", hits ? hits.length : 0);
  
  if (hits && hits.length > 0) {
    console.log("✅ SUCCESS: API found the freshly copied item!");
  } else {
    console.log("❌ FAILURE: API did NOT find the freshly copied item. index.json might not have updated correctly.");
  }
}

benchmark().catch(console.error);
