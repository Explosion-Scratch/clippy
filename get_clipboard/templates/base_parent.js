window.addEventListener('message', function (event) {
    const data = event.data;
    if (data.type === 'copy') {
        // Use Tauri invoke or navigator.clipboard
        if (window.__TAURI__) {
            window.__TAURI__.core.invoke('copy_to_clipboard', { text: data.text }).catch(console.error);
        } else {
            navigator.clipboard.writeText(data.text).catch(console.error);
        }
    } else if (data.type === 'toast') {
        // Dispatch custom event for Vue app to handle
        window.dispatchEvent(new CustomEvent('show-toast', { detail: data.toast }));
    }
});
