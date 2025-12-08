
export function showToast(text, options = {}) {
    const timeout = options.timeout || 3000;
    const bottomPos = options.bottom || "20px";
    
    let t = document.querySelector("#app_toast");
    
    // Check if a toast is already shown and just update it
    if (Date.now() < (window.toast_time || 0) && t) {
        t.innerText = text;
        if (window.toast_1_int) clearTimeout(window.toast_1_int);
        if (window.toast_2_int) clearTimeout(window.toast_2_int);
        
        t.style.bottom = bottomPos;
        
        window.toast_time = Date.now() + (timeout - 600);
        window.toast_1_int = setTimeout(() => (t.style.bottom = "-200px"), timeout - 500);
        window.toast_2_int = setTimeout(() => t?.remove(), timeout);
        return;
    }
    
    t?.remove();
    
    t = document.createElement("div");
    t.id = "app_toast";
    t.setAttribute("style", `
        position: fixed;
        bottom: -200px;
        z-index: 1000000000;
        transition: bottom .5s cubic-bezier(.44,.57,.44,1.25);
        border-radius: 1000px;
        background: #000a;
        border: 1px solid #0009;
        color: white;
        display: flex;
        justify-content: center;
        align-items: center;
        padding: 4px 15px;
        left: 50vw;
        transform: translate(-50%, -50%);
        font-family: system-ui, sans-serif;
        font-size: 12px;
        width: fit-content;
        pointer-events: none;
    `);
    
    document.body.appendChild(t);
    t.innerText = text;
    
    // Trigger animation
    setTimeout(() => { t.style.bottom = bottomPos; }, 10);
    
    window.toast_time = Date.now() + (timeout - 600);
    window.toast_1_int = setTimeout(() => (t.style.bottom = "-200px"), timeout - 500);
    window.toast_2_int = setTimeout(() => t?.remove(), timeout);
}
