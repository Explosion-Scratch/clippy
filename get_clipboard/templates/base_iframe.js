window.copy = function (text) {
    window.parent.postMessage({ type: 'copy', text: text }, '*');
};

window.toast = function (toastObj) {
    window.parent.postMessage({ type: 'toast', toast: toastObj }, '*');
};

// Helper for copy buttons
document.addEventListener('click', function (e) {
    if (e.target.closest('.copy-btn')) {
        const btn = e.target.closest('.copy-btn');
        const text = btn.dataset.text;
        if (text) {
            window.copy(text);
            window.toast({ message: 'Copied to clipboard', type: 'success' });
        }
    }
});
