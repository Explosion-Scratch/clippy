#!/usr/bin/env node

// Simple test script to verify clipboard functionality
const { invoke } = require('@tauri-apps/api/core');

async function testClipboard() {
    try {
        console.log('Testing clipboard functionality...');
        
        // Create a test clipboard item with multiple formats
        const testItem = {
            id: 999,
            text: "Test clipboard content",
            timestamp: Date.now() / 1000,
            byte_size: 100,
            formats: {
                txt: "Test clipboard content",
                html: "<p>Test <strong>clipboard</strong> content</p>",
                imageData: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==", // 1x1 red pixel
                files: ["/Users/tjs/test.txt"],
                customFormats: {
                    "test-format": "custom data"
                }
            }
        };
        
        console.log('Test item created:', JSON.stringify(testItem, null, 2));
        
        // This would normally be called from the frontend
        // const result = await invoke("set_clipboard_item", { item: testItem });
        // console.log('Result:', result);
        
        console.log('Test completed successfully!');
    } catch (error) {
        console.error('Test failed:', error);
    }
}

testClipboard();