<script setup>
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import ClipboardItem from "./ClipboardItem.vue";
import InlinePreview from "./InlinePreview.vue";
import { showToast } from "../utils/ui";

const router = useRouter();

const BATCH_SIZE = 30;
const WINDOW_STATE_KEY = 'clipboardManagerWindowState';
const INLINE_PREVIEW_MIN_WIDTH = 500;
const DIR_CHECK_DELAY_MS = 1000;
const HOVER_LOCK_DURATION_MS = 180;
const LOADING_INDICATOR_DELAY_MS = 300;
const POLL_INTERVAL_MS = 1500;
const RESIZE_DEBOUNCE_MS = 50;

const allLoadedItems = ref([]);
const searchQuery = ref("");
const isLoading = ref(false);
const isLoadingMore = ref(false);
const showLoadingStatus = ref(false);
const globalSelectedIndex = ref(-1);
const selectedIndex = ref(-1);
const clipboardManager = ref(null);
const clipboardListRef = ref(null);
const loadMoreSentinel = ref(null);
const searchInputRef = ref(null);
const clipboardItems = computed(() => allLoadedItems.value);
const totalItems = ref(0);
const windowWidth = ref(400);
const windowHeight = ref(400);

let loadingTimer = null;
let windowResizeObserver = null;
let pollingInterval = null;
let lastKnownId = null;
let loadMoreObserver = null;
let listScrollCleanup = null;
let hoverLockUntil = 0;
let keydownHandler = null;
let keyupHandler = null;
let resizeDebounceTimer = null;
let focusUnlisten = null;
let previewChangedUnlisten = null;

const showInlinePreview = computed(() => windowWidth.value >= INLINE_PREVIEW_MIN_WIDTH);

const showDirModal = ref(false);
const mismatchDirs = ref({ current: "", expected: "" });

const isCycling = ref(false);
const isModifierPressed = ref(false);

const configuredShortcut = ref({ ctrl: true, alt: false, shift: false, meta: false, code: 'KeyP' });

const codeToKeyMap = {
  KeyA: 'A', KeyB: 'B', KeyC: 'C', KeyD: 'D', KeyE: 'E',
  KeyF: 'F', KeyG: 'G', KeyH: 'H', KeyI: 'I', KeyJ: 'J',
  KeyK: 'K', KeyL: 'L', KeyM: 'M', KeyN: 'N', KeyO: 'O',
  KeyP: 'P', KeyQ: 'Q', KeyR: 'R', KeyS: 'S', KeyT: 'T',
  KeyU: 'U', KeyV: 'V', KeyW: 'W', KeyX: 'X', KeyY: 'Y',
  KeyZ: 'Z',
  Digit0: '0', Digit1: '1', Digit2: '2', Digit3: '3', Digit4: '4',
  Digit5: '5', Digit6: '6', Digit7: '7', Digit8: '8', Digit9: '9',
  F1: 'F1', F2: 'F2', F3: 'F3', F4: 'F4', F5: 'F5', F6: 'F6',
  F7: 'F7', F8: 'F8', F9: 'F9', F10: 'F10', F11: 'F11', F12: 'F12',
};

function parseShortcutString(shortcutStr) {
  const parts = shortcutStr.split('+');
  const result = { ctrl: false, alt: false, shift: false, meta: false, code: 'KeyP' };
  
  for (const part of parts) {
    switch (part) {
      case 'Control': case 'Ctrl': result.ctrl = true; break;
      case 'Alt': case 'Option': result.alt = true; break;
      case 'Shift': result.shift = true; break;
      case 'Meta': case 'Cmd': case 'Command': result.meta = true; break;
      default:
        for (const [code, key] of Object.entries(codeToKeyMap)) {
          if (key === part.toUpperCase()) {
            result.code = code;
            break;
          }
        }
        break;
    }
  }
  
  return result;
}

async function loadConfiguredShortcut() {
  try {
    const shortcutStr = await invoke('get_configured_shortcut');
    configuredShortcut.value = parseShortcutString(shortcutStr);
  } catch (e) {
    console.error('Failed to load configured shortcut:', e);
  }
}

function matchesConfiguredShortcut(e) {
  const s = configuredShortcut.value;
  return e.ctrlKey === s.ctrl &&
         e.altKey === s.alt &&
         e.shiftKey === s.shift &&
         e.metaKey === s.meta &&
         e.code === s.code;
}

function hasAnyModifierPressed(e) {
  const s = configuredShortcut.value;
  return (s.ctrl && e.ctrlKey) ||
         (s.alt && e.altKey) ||
         (s.shift && e.shiftKey) ||
         (s.meta && e.metaKey);
}

const selectedItem = computed(() => {
    if (globalSelectedIndex.value >= 0 && allLoadedItems.value[globalSelectedIndex.value]) {
        return allLoadedItems.value[globalSelectedIndex.value];
    }
    return null;
});

const searchPlaceholder = computed(() => `Search ${totalItems.value} items`);

async function saveWindowState() {
    try {
        const win = getCurrentWindow();
        const pos = await win.outerPosition();
        const size = await win.outerSize();
        localStorage.setItem(WINDOW_STATE_KEY, JSON.stringify({
            x: pos.x, y: pos.y, 
            width: size.width, height: size.height
        }));
    } catch (e) {
        console.error('Failed to save window state:', e);
    }
}

async function restoreWindowState() {
    try {
        const saved = localStorage.getItem(WINDOW_STATE_KEY);
        if (saved) {
            const { x, y, width, height } = JSON.parse(saved);
            const win = getCurrentWindow();
            await win.setPosition(new PhysicalPosition(x, y));
            await win.setSize(new PhysicalSize(width, height));
        }
    } catch (e) {
        console.error('Failed to restore window state:', e);
    }
}

async function checkDataDirectory() {
    try {
        if (localStorage.getItem('ignoreDirMismatch') === 'true') return;
        await new Promise(resolve => setTimeout(resolve, DIR_CHECK_DELAY_MS));
        const currentDir = await invoke('get_sidecar_dir');
        const expectedDir = await invoke('get_app_data_dir');
        
        if (currentDir && expectedDir && currentDir !== expectedDir) {
            mismatchDirs.value = { current: currentDir, expected: expectedDir };
            showDirModal.value = true;
            const window = getCurrentWindow();
            await window.setSize(new LogicalSize(400, 400));
        }
    } catch (e) {
        console.error("Failed to check directory:", e);
    }
}

async function handleDirChoice(choice) {
    try {
        if (choice === 'continue') {
            localStorage.setItem('ignoreDirMismatch', 'true');
            showDirModal.value = false;
            loadItems();
            return;
        }
        const path = mismatchDirs.value.expected;
        if (choice === 'create') {
            await invoke('set_sidecar_dir', { mode: 'update', path });
        } else if (choice === 'import') {
            await invoke('set_sidecar_dir', { mode: 'move', path });
        }
        showDirModal.value = false;
        loadItems();
    } catch (e) {
        console.error("Failed to set directory:", e);
        alert("Failed to update directory: " + e);
    }
}

function startLoading() {
    isLoading.value = true;
    showLoadingStatus.value = false;
    if (loadingTimer) clearTimeout(loadingTimer);
    loadingTimer = setTimeout(() => { showLoadingStatus.value = true; }, LOADING_INDICATOR_DELAY_MS);
}

function stopLoading() {
    isLoading.value = false;
    showLoadingStatus.value = false;
    if (loadingTimer) { 
        clearTimeout(loadingTimer); 
        loadingTimer = null; 
    }
}

async function loadItems(appendMode = false, preserveId = null) {
    try {
        if (!appendMode) startLoading();
        else isLoadingMore.value = true;
        
        const query = searchQuery.value.trim();
        const offset = appendMode ? allLoadedItems.value.length : 0;
        const jsonStr = await invoke("get_history", { 
            query: query || null, 
            limit: BATCH_SIZE, 
            offset: offset 
        });
        
        const rawItems = JSON.parse(jsonStr);
        const items = rawItems.map(mapApiItem);
        const targetId = preserveId || (appendMode ? null : selectedItem.value?.id);
        
        if (!appendMode) {
            allLoadedItems.value = items;
            if (items.length > 0) lastKnownId = items[0].id;

            if (isCycling.value) {
                // Don't reset selection during cycling mode
            } else if (targetId) {
                const idx = items.findIndex((it) => it.id === targetId);
                globalSelectedIndex.value = idx >= 0 ? idx : (items.length > 0 ? 0 : -1);
            } else {
                globalSelectedIndex.value = items.length > 0 ? 0 : -1;
            }
            
            if (!isCycling.value) {
                await syncPreviewWindow();
            }
        } else {
            allLoadedItems.value = [...allLoadedItems.value, ...items];
        }
    } catch (error) {
        console.error("Failed to load items:", error);
    } finally {
        stopLoading();
        isLoadingMore.value = false;
    }
}

async function loadMoreItems() {
    if (isLoadingMore.value || allLoadedItems.value.length >= totalItems.value) return;
    await loadItems(true);
}

async function loadTotalItems() {
    try {
        const count = await invoke("db_get_count");
        totalItems.value = count;
    } catch (e) {
        console.error("Failed to load total items:", e);
        totalItems.value = 0;
    }
}

async function syncPreviewWindow() {
    if (!selectedItem.value) return;
    await showPreview(selectedItem.value.id.toString());
}

function mapApiItem(item) {
    const id = item.hash || item.id;
    const idx = item.offset !== undefined ? item.offset : item.index;
    const itemType = item.type;

    return {
        id: id,
        index: idx,
        text: item.summary,
        timestamp: new Date(item.lastSeen || item.date || Date.now()).getTime(),
        byteSize: item.byteSize || item.size || 0,
        copies: item.copyCount || 0,
        firstCopied: item.timestamp ? new Date(item.timestamp).getTime() / 1000 : (new Date(item.date).getTime() / 1000),
        data: item.data,
        formats: {
            imageData: itemType === "image",
            files: (itemType === "file" || itemType === "files") ? [item.summary] : [],
        }
    };
}

watch(selectedItem, async (newItem) => {
    if (!newItem) return;
    await showPreview(newItem.id.toString());
});

watch(showInlinePreview, async () => {
    if (!selectedItem.value) return;
    await showPreview(selectedItem.value.id.toString());
});

async function showPreview(id) {
    if (!id) {
        await hidePreview();
        return;
    }
    if (showInlinePreview.value) {
        await hidePreview();
    } else {
        try {
            await invoke("preview_item", { id: id.toString() });
        } catch (e) {
            console.error("Failed to show preview:", e);
        }
    }
}

async function hidePreview() {
    try {
        await invoke("hide_preview");
    } catch (e) {
        console.error("Failed to hide preview window:", e);
    }
}

function handleToastEvent(e) {
    const detail = e.detail;
    if (typeof detail === 'string') {
        showToast(detail, { bottom: "20px" });
    } else if (detail?.message) {
        showToast(detail.message, { timeout: detail.timeout || 3000, bottom: "20px" });
    }
}

watch(searchQuery, () => {
    globalSelectedIndex.value = -1;
    loadItems();
}, { debounce: 300 });

watch(loadMoreSentinel, (el, prev) => {
    if (!loadMoreObserver) return;
    if (prev) loadMoreObserver.unobserve(prev);
    if (el) loadMoreObserver.observe(el);
});

async function deleteItem(id) {
    try {
        await invoke("delete_item", { selector: id.toString() });
        allLoadedItems.value = allLoadedItems.value.filter(item => item.id !== id);
        if (globalSelectedIndex.value >= allLoadedItems.value.length) {
            globalSelectedIndex.value = Math.max(0, allLoadedItems.value.length - 1);
        }
    } catch (error) {
        console.error("Failed to delete item:", error);
    }
}

async function pollForChanges() {
    try {
        const mtimeJson = await invoke("get_mtime");
        const mtime = JSON.parse(mtimeJson);
        if (mtime.id && mtime.id !== lastKnownId && !searchQuery.value) {
            await loadItems(false, selectedItem.value?.id);
            await loadTotalItems();
        }
    } catch (e) {
    }
}

function handleSearchKeyDown(e) {
    if (matchesConfiguredShortcut(e)) {
        e.preventDefault();
    }
}

function handleKeyDown(e) {
    if (matchesConfiguredShortcut(e)) {
        e.preventDefault();
        if (!isCycling.value) startCyclingMode();
        else cycleToNext();
        return;
    }
    
    if (hasAnyModifierPressed(e)) {
        isModifierPressed.value = true;
    }
    
    if (e.metaKey && !e.shiftKey && !e.altKey && !e.ctrlKey) {
        const key = e.key;
        
        if (key === ",") {
            e.preventDefault();
            invoke("open_settings").catch(err => {
                console.error("Failed to open settings:", err);
            });
            return;
        }
        
        let itemIndex = null;
        if (key >= "1" && key <= "9") itemIndex = parseInt(key) - 1;
        else if (key === "0") itemIndex = 9;
        if (itemIndex !== null && clipboardItems.value[itemIndex]) {
            e.preventDefault();
            pasteItemToSystem(clipboardItems.value[itemIndex]);
        }
    }
    
    if (!isCycling.value) {
        if (e.key === "ArrowDown") { 
            e.preventDefault(); 
            handleArrowDown(); 
        } else if (e.key === "ArrowUp") { 
            e.preventDefault(); 
            handleArrowUp(); 
        } else if (e.key === "Enter") {
            e.preventDefault();
            if (e.shiftKey) {
                if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
                    invoke("open_in_dashboard", { id: clipboardItems.value[selectedIndex.value].id.toString() })
                        .catch(err => console.error("Failed to open in dashboard:", err));
                    invoke("hide_app");
                }
            } else if (e.metaKey) {
                if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
                    copyItemToSystem(clipboardItems.value[selectedIndex.value]);
                }
            } else {
                handleEnter();
            }
        }
    }
}

function handleKeyUp(e) {
    const s = configuredShortcut.value;
    const modifierReleased = 
        (s.ctrl && e.key === 'Control') ||
        (s.alt && e.key === 'Alt') ||
        (s.shift && e.key === 'Shift') ||
        (s.meta && e.key === 'Meta');
    
    if (modifierReleased) {
        isModifierPressed.value = false;
        if (isCycling.value) endCycling();
    }
    if (e.key === "Escape") {
        if (showDirModal.value) {
            localStorage.setItem('ignoreDirMismatch', 'true');
            showDirModal.value = false;
            loadItems();
            return;
        }
        invoke("hide_app");
    }
}

function scrollSelectedIntoView() {
    nextTick(() => {
        const selectedEl = clipboardListRef.value?.querySelector('.clipboard-item.is-selected');
        if (selectedEl) {
            selectedEl.scrollIntoView({ block: 'nearest', behavior: 'auto' });
        }
    });
}

function lockHoverSelection() {
    hoverLockUntil = Date.now() + HOVER_LOCK_DURATION_MS;
}

function handleMouseEnter(index) {
    if (Date.now() < hoverLockUntil) return;
    globalSelectedIndex.value = index;
}

async function handleArrowDown() {
    if (allLoadedItems.value.length === 0) return;
    lockHoverSelection();
    
    if (globalSelectedIndex.value < allLoadedItems.value.length - 1) {
        globalSelectedIndex.value++;
        scrollSelectedIntoView();
        
        if (globalSelectedIndex.value >= allLoadedItems.value.length - 5) {
            await loadMoreItems();
        }
    }
}

function handleArrowUp() {
    if (allLoadedItems.value.length === 0) return;
    lockHoverSelection();
    
    if (globalSelectedIndex.value > 0) {
        globalSelectedIndex.value--;
        scrollSelectedIntoView();
    }
}

function handleEnter() {
    if (selectedItem.value) {
        pasteItemToSystem(selectedItem.value);
    }
}

async function pasteItemToSystem(item) {
    try {
        await invoke("hide_app");
        await invoke("paste_item", { selector: item.id.toString() });
        loadItems(false, item.id);
    } catch (error) {
        console.error("Failed to inject item:", error);
    }
}

async function copyItemToSystem(item) {
    try {
        await invoke("copy_item", { selector: item.id.toString() });
        await invoke("hide_app");
    } catch (error) {
        console.error("Failed to copy item:", error);
    }
}

async function refreshItem(id) {
    const targetId = id || selectedItem.value?.id;
    if (!targetId) return;
    await loadItems(false, targetId);
}

function resetSelection() {
    globalSelectedIndex.value = -1;
    searchQuery.value = "";
}

function formatFirstCopied(firstCopied) {
    const date = new Date(firstCopied * 1000);
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    const hours = date.getHours();
    const minutes = String(date.getMinutes()).padStart(2, "0");
    const ampm = hours >= 12 ? "PM" : "AM";
    const displayHours = hours % 12 || 12;
    return `${month}/${day} @ ${displayHours}:${minutes}${ampm}`;
}

function getItemInfo(item) {
    if (!item) return null;
    if (item.formats?.imageData) return { type: "image", size: item.byteSize, label: "Image" };
    if (item.formats?.files && item.formats.files.length > 0) return { type: "files", size: `${item.formats.files.length} files`, label: "Files" };
    const wordCount = item.text ? item.text.trim().split(/\s+/).length : 0;
    return { type: "text", size: `${wordCount} words`, label: "Text" };
}

async function unregisterGlobalShortcut() { 
    await invoke("unregister_main_shortcut").catch(console.error); 
}

async function registerGlobalShortcut() { 
    await invoke("register_main_shortcut").catch(console.error); 
}

function startCyclingMode() {
    if (clipboardItems.value.length === 0) return;
    isCycling.value = true;
    selectedIndex.value = clipboardItems.value.length > 1 ? 1 : 0;
    globalSelectedIndex.value = selectedIndex.value;
    scrollSelectedIntoView();
}

function cycleToNext() {
    if (!isCycling.value || clipboardItems.value.length === 0) return;
    selectedIndex.value = (selectedIndex.value + 1) % clipboardItems.value.length;
    globalSelectedIndex.value = selectedIndex.value;
    scrollSelectedIntoView();
}

async function endCycling() {
    if (!isCycling.value) return;
    isCycling.value = false;
    globalSelectedIndex.value = selectedIndex.value;
    if (selectedIndex.value >= 0 && clipboardItems.value[selectedIndex.value]) {
        await pasteItemToSystem(clipboardItems.value[selectedIndex.value]);
    }
    selectedIndex.value = -1;
}

function handleDebouncedResize(width, height) {
    if (resizeDebounceTimer) {
        clearTimeout(resizeDebounceTimer);
    }
    resizeDebounceTimer = setTimeout(() => {
        windowWidth.value = width;
        windowHeight.value = height;
        resizeDebounceTimer = null;
    }, RESIZE_DEBOUNCE_MS);
}

async function handleFocusChange(focused) {
    if (!focused) {
        if (isCycling.value) {
            isCycling.value = false;
            isCtrlPressed.value = false;
        }
        registerGlobalShortcut();
        saveWindowState();
    } else {
        unregisterGlobalShortcut();
        if (!isCycling.value) {
            if (globalSelectedIndex.value === -1 && allLoadedItems.value.length > 0) {
                globalSelectedIndex.value = 0;
            }
            loadTotalItems();
            loadItems(false, selectedItem.value?.id || allLoadedItems.value[0]?.id).then(() => {
                nextTick(() => searchInputRef.value?.focus());
            });
        }
        isCtrlPressed.value = true;
    }
}

function cleanup() {
    if (keydownHandler) {
        document.removeEventListener("keydown", keydownHandler);
        keydownHandler = null;
    }
    if (keyupHandler) {
        document.removeEventListener("keyup", keyupHandler);
        keyupHandler = null;
    }
    if (focusUnlisten) {
        focusUnlisten();
        focusUnlisten = null;
    }
    if (previewChangedUnlisten) {
        previewChangedUnlisten();
        previewChangedUnlisten = null;
    }
    if (windowResizeObserver) {
        windowResizeObserver.disconnect();
        windowResizeObserver = null;
    }
    if (loadMoreObserver) {
        loadMoreObserver.disconnect();
        loadMoreObserver = null;
    }
    if (pollingInterval) {
        clearInterval(pollingInterval);
        pollingInterval = null;
    }
    if (listScrollCleanup) {
        listScrollCleanup();
        listScrollCleanup = null;
    }
    if (loadingTimer) {
        clearTimeout(loadingTimer);
        loadingTimer = null;
    }
    if (resizeDebounceTimer) {
        clearTimeout(resizeDebounceTimer);
        resizeDebounceTimer = null;
    }
    window.removeEventListener('show-toast', handleToastEvent);
}

onMounted(async () => {
    try {
        const isFirstRun = await invoke('check_first_run');
        if (isFirstRun) {
            router.replace('/welcome');
            return;
        }
    } catch (e) {
        console.error('Failed to check first run:', e);
    }
    
    await loadConfiguredShortcut();
    await restoreWindowState();
    
    try {
        const size = await getCurrentWindow().outerSize();
        windowWidth.value = size.width;
        windowHeight.value = size.height;
    } catch (e) {
        console.error("Failed to read window size:", e);
    }
    
    keydownHandler = handleKeyDown;
    keyupHandler = handleKeyUp;
    document.addEventListener("keydown", keydownHandler);
    document.addEventListener("keyup", keyupHandler);
    
    window.addEventListener('show-toast', handleToastEvent);
    
    focusUnlisten = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
        handleFocusChange(focused);
    });

    previewChangedUnlisten = await listen("preview-item-changed", async (event) => {
        const newId = event.payload;
        if (newId) {
            await loadItems(false, newId);
        }
    });

    await loadTotalItems();
    await loadItems();
    await checkDataDirectory();

    await nextTick();
    searchInputRef.value?.focus();

    windowResizeObserver = new ResizeObserver((entries) => {
        const entry = entries[0];
        if (entry) {
            handleDebouncedResize(entry.contentRect.width, entry.contentRect.height);
        }
    });
    if (clipboardManager.value) {
        windowResizeObserver.observe(clipboardManager.value);
    }

    loadMoreObserver = new IntersectionObserver((entries) => {
        if (entries[0].isIntersecting && !isLoadingMore.value) {
            loadMoreItems();
        }
    }, { threshold: 0.1, root: clipboardListRef.value || undefined });

    if (loadMoreObserver && loadMoreSentinel.value) {
        loadMoreObserver.observe(loadMoreSentinel.value);
    }

    if (clipboardListRef.value) {
        const handleListScroll = () => {
            const el = clipboardListRef.value;
            if (!el) return;
            if (el.scrollTop + el.clientHeight >= el.scrollHeight - 8) {
                loadMoreItems();
            }
        };
        clipboardListRef.value.addEventListener("scroll", handleListScroll, { passive: true });
        listScrollCleanup = () => {
            clipboardListRef.value?.removeEventListener("scroll", handleListScroll);
        };
    }

    pollingInterval = setInterval(pollForChanges, POLL_INTERVAL_MS);
});

onUnmounted(() => {
    cleanup();
});
</script>

<template>
    <div class="clipboard-manager" ref="clipboardManager">
        <div v-if="showDirModal" class="modal-overlay">
            <div class="modal">
                <h3>Storage Location</h3>
                <p>The clipboard data directory differs from the recommended application support directory.</p>
                <div class="paths">
                    <div class="path-item"><strong>Current:</strong> {{ mismatchDirs.current }}</div>
                    <div class="path-item"><strong>Recommended:</strong> {{ mismatchDirs.expected }}</div>
                </div>
                <div class="modal-actions">
                    <button @click="handleDirChoice('create')">Create new</button>
                    <button @click="handleDirChoice('import')">Import</button>
                    <button @click="handleDirChoice('continue')" class="secondary">Continue</button>
                </div>
            </div>
        </div>

        <div class="search-container">
            <input ref="searchInputRef" autocapitalize="off" autocomplete="off" autocorrect="off" spellcheck="off" v-model="searchQuery" type="text" :placeholder="searchPlaceholder" class="search-input" @keydown="handleSearchKeyDown" />
        </div>

        <div class="content-area" :class="{ 'has-preview': showInlinePreview && selectedItem }">
            <div class="items-container">
                <div v-if="allLoadedItems?.length === 0 && !isLoading" class="empty-state">
                    <div class="empty-icon">
                        <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 256 256"><path fill="currentColor" d="M200,32H163.74a47.92,47.92,0,0,0-71.48,0H56A16,16,0,0,0,40,48V216a16,16,0,0,0,16,16H200a16,16,0,0,0,16-16V48A16,16,0,0,0,200,32Zm-72,0a32,32,0,0,1,32,32H96A32,32,0,0,1,128,32Zm72,184H56V48H82.75A47.93,47.93,0,0,0,80,64v8a8,8,0,0,0,8,8h80a8,8,0,0,0,8-8V64a47.93,47.93,0,0,0-2.75-16H200Z"/></svg>
                    </div>
                    <p>{{ searchQuery ? "No results" : "Copy something to get started" }}</p>
                </div>
                <div v-else class="clipboard-list" ref="clipboardListRef">
                    <ClipboardItem 
                        v-for="(item, index) in allLoadedItems" 
                        :key="item.id" 
                        :item="{ ...item, index }" 
                        :selected="index === globalSelectedIndex" 
                        @mouseenter="handleMouseEnter(index)" 
                        @delete="deleteItem(item.id)" 
                        @select="pasteItemToSystem(item)" 
                    />
                    <div v-if="allLoadedItems.length < totalItems" ref="loadMoreSentinel" class="load-more-sentinel">
                        <div v-if="isLoadingMore" class="loading-more">
                            <div class="spinner"></div>
                        </div>
                    </div>
                </div>
            </div>
            <div v-if="showInlinePreview && selectedItem" class="inline-preview">
                <InlinePreview :itemId="selectedItem.id" @refresh="refreshItem" />
            </div>
        </div>

        <div class="status-bar">
            <div v-if="showLoadingStatus" class="status-item loading-status">
                <div class="spinner"></div><span class="status-value">Loading...</span>
            </div>
            <template v-else-if="selectedItem">
                <div class="status-item"><span class="status-value">{{ formatFirstCopied(selectedItem.firstCopied) }}</span></div>
                <div class="status-item"><span class="status-value">{{ selectedItem.copies }} copies</span></div>
            </template>
            <template v-else>
                <div class="status-item"><span class="status-value">{{ totalItems }} items</span></div>
            </template>
        </div>
    </div>
</template>

<style lang="less">
.clipboard-manager {
    display: flex;
    flex-direction: column;
    font-family: system-ui, sans-serif;
    font-weight: normal;
    gap: 10px;
    padding: 8px;
    background: var(--bg-primary);
    color: var(--text-primary);
    height: 100vh;
    max-height: 100vh;
    overflow: hidden;

    .search-container { margin-top: 3px; }
    .content-area {
        display: flex;
        gap: 10px;
        flex: 1;
        min-height: 0;
        overflow: hidden;
    }
    .items-container {
        flex: 1;
        display: flex;
        flex-direction: column;
        min-width: 0;
        overflow: hidden;
    }
    .content-area.has-preview {
        .items-container {
            flex: 0 0 45%;
            max-width: 45%;
        }
    }
    .inline-preview {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        background: var(--bg-secondary);
        border-radius: 4px;
        overflow: hidden;
    }

    .clipboard-list {
        padding-top: 10px; 
        display: flex; 
        flex-direction: column; 
        gap: 1px;
        flex: 1;
        overflow-y: auto;
        scrollbar-width: none;
        &::-webkit-scrollbar { display: none; }
        .clipboard-item {
            height: 23px; overflow: hidden; cursor: default; font-size: 0.8em; display: flex; justify-content: space-between; gap: 10px; align-items: center; border-radius: 4px; padding: 1px 5px; color: var(--text-primary);
            flex-shrink: 0;
            .info { opacity: 0.6; color: var(--text-secondary); }
            &.is-selected { background: var(--accent); color: var(--accent-text); .info { color: var(--accent-text); opacity: 0.8; } }
        }
        .clipboard-item:has(img) { height: 80px; padding-top: 4px; padding-bottom: 4px; img { height: calc(100% - 8px); } }
        .load-more-sentinel {
            height: 30px;
            flex-shrink: 0;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .loading-more {
            .spinner {
                width: 16px;
                height: 16px;
                border: 2px solid var(--border-color);
                border-radius: 50%;
                border-top-color: var(--accent);
                animation: spin 1s linear infinite;
            }
        }
    }
}
.modal-overlay { position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0, 0, 0, 0.5); display: flex; justify-content: center; align-items: center; z-index: 1000; padding: 20px; backdrop-filter: blur(5px); }
.modal { background: white; border-radius: 8px; padding: 20px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); width: 100%; max-width: 400px; color: #333; h3 { margin-top: 0; } .paths { background: #f5f5f5; padding: 10px; border-radius: 4px; font-size: 0.8em; margin: 15px 0; word-break: break-all; .path-item { margin-bottom: 8px; &:last-child { margin-bottom: 0; } } } .modal-actions { display: flex; flex-direction: column; gap: 8px; button { padding: 8px; border-radius: 4px; border: none; background: var(--accent); color: white; cursor: pointer; font-weight: bold; &:hover { filter: brightness(1.1); } &.secondary { background: #e0e0e0; color: #333; } } } }
.search-input { background: var(--bg-input); border: 0.5px solid var(--border-light); border-radius: 5px; padding: 5px 8px; font-family: system-ui; box-shadow: var(--shadow-light); color: var(--text-primary); width: 100%; &::placeholder { color: var(--text-secondary); opacity: 0.7; } &:focus { outline: none; border: none; box-shadow: 0 0 0 3px var(--accent-transparent); } }
@keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
.empty-state { text-align: center; color: var(--text-secondary); }
.empty-icon { font-size: 32px; filter: grayscale(0.3); }
.status-bar { display: flex; align-items: center; padding: 4px 12px; background: var(--bg-status); color: var(--text-secondary); border-radius: 4px; font-size: 0.75em; margin-top: auto; margin-bottom: 4px; flex-shrink: 0; height: 20px; line-height: 20px; }
.status-item { display: flex; align-items: center; gap: 4px; flex: 1; justify-content: center; }
.status-value { font-weight: 300; color: var(--text-secondary); }
.loading-status { justify-content: center; gap: 6px; }
.loading-status .spinner { width: 12px; height: 12px; border: 1.5px solid var(--border-color); border-radius: 50%; border-top: none; animation: spin 1s linear infinite; }
</style>
