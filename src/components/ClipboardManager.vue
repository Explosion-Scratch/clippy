<script setup>
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize, PhysicalPosition, PhysicalSize } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";
import ClipboardItem from "./ClipboardItem.vue";
import InlinePreview from "./InlinePreview.vue";
import { showToast } from "../utils/ui";
import { handleItemShortcuts, getFilteredShortcuts } from "../utils/itemShortcuts";
import {
    useClipboardItems,
    useKeyboardHandling,
    useClipboardCycling,
    useWindowFocus,
    useClipboardPreview,
    useClipboardActions,
    useClipboardChangeEvent,
    usePreviewChangeEvent,
    useListSelection
} from "../composables";

const router = useRouter();

const WINDOW_STATE_KEY = 'clipboardManagerWindowState';
const INLINE_PREVIEW_MIN_WIDTH = 500;
const DIR_CHECK_DELAY_MS = 1000;
const LOADING_INDICATOR_DELAY_MS = 300;
const RESIZE_DEBOUNCE_MS = 50;

const clipboardManager = ref(null);
const clipboardListRef = ref(null);
const loadMoreSentinel = ref(null);
const searchInputRef = ref(null);
const inlinePreviewRef = ref(null);
const windowWidth = ref(400);
const windowHeight = ref(400);
const showLoadingStatus = ref(false);

let loadingTimer = null;
let windowResizeObserver = null;
let loadMoreObserver = null;
let listScrollCleanup = null;
let resizeDebounceTimer = null;
let lastKnownId = null;

const showInlinePreview = computed(() => windowWidth.value >= INLINE_PREVIEW_MIN_WIDTH);
const showDirModal = ref(false);
const mismatchDirs = ref({ current: "", expected: "" });

const {
    items: clipboardItems,
    allLoadedItems,
    totalItems,
    isLoading,
    isLoadingMore,
    searchQuery,
    apiStatus,
    hasMore,
    isEmpty,
    loadItems: loadItemsBase,
    loadMore,
    loadTotal,
    deleteItem: deleteItemBase,
    restartApi: restartApiBase,
    dismissApiError
} = useClipboardItems();

const {
    selectedIndex: globalSelectedIndex,
    selectedItem,
    selectNext,
    selectPrev,
    selectIndex,
    selectFirst,
    handleMouseEnter,
    scrollIntoView,
    reset: resetSelection
} = useListSelection(allLoadedItems, {
    listRef: clipboardListRef,
    initialIndex: -1,
    onLoadMore: loadMore
});

const {
    configuredShortcut,
    isModifierPressed,
    state: keyboardState,
    loadConfiguredShortcut,
    matchesConfiguredShortcut,
    hasAnyShortcutModifier,
    updateState: updateKeyboardState,
    resetState: resetKeyboardState,
    unregisterGlobalShortcut,
    registerGlobalShortcut
} = useKeyboardHandling({ 
    autoRegister: false,
    emitToPreview: computed(() => !showInlinePreview.value)
});

const {
    pasteItem,
    copyItem,
    pasteAsPlainText,
    copyAsPlainText,
    hideApp,
    openSettings
} = useClipboardActions();

const preview = useClipboardPreview({
    useInlinePreview: showInlinePreview
});

const {
    isCycling,
    activeIndex: cyclingIndex,
    startCycling,
    cycleNext,
    endCycling,
    cancelCycling
} = useClipboardCycling(clipboardItems, {
    onCycleStart: (item, index) => {
        globalSelectedIndex.value = index;
        scrollIntoView();
    },
    onSelect: (item, index) => {
        globalSelectedIndex.value = index;
        scrollIntoView();
    },
    onCycleEnd: async (item) => {
        await pasteItemToSystem(item);
    }
});

const { isFocused: isWindowFocused } = useWindowFocus({
    onFocus: async () => {
        unregisterGlobalShortcut();
        searchQuery.value = "";
        selectFirst();
        if (clipboardListRef.value) clipboardListRef.value.scrollTop = 0;
        if (inlinePreviewRef.value) inlinePreviewRef.value.resetState?.();
        nextTick(() => searchInputRef.value?.focus());
        isModifierPressed.value = true;
    },
    onBlur: async () => {
        searchQuery.value = "";
        selectFirst();
        if (clipboardListRef.value) clipboardListRef.value.scrollTop = 0;
        if (inlinePreviewRef.value) inlinePreviewRef.value.resetState?.();
        await preview.hide();
        if (isCycling.value) {
            cancelCycling();
            isModifierPressed.value = false;
        }
        resetKeyboardState();
        registerGlobalShortcut();
        saveWindowState();
    }
});

useClipboardChangeEvent(async (newId) => {
    if (apiStatus.value === 'error') {
        apiStatus.value = 'connected';
    }
    if (newId && newId !== lastKnownId && !searchQuery.value) {
        lastKnownId = newId;
        const preserveId = isWindowFocused.value ? selectedItem.value?.id : null;
        await loadItems(false, preserveId);
        await loadTotal();
    }
});

usePreviewChangeEvent(async (newId) => {
    if (newId) {
        await loadItems(false, newId);
    }
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
    if (loadingTimer) clearTimeout(loadingTimer);
    loadingTimer = setTimeout(() => { showLoadingStatus.value = true; }, LOADING_INDICATOR_DELAY_MS);
}

function stopLoading() {
    showLoadingStatus.value = false;
    if (loadingTimer) { 
        clearTimeout(loadingTimer); 
        loadingTimer = null; 
    }
}

async function loadItems(appendMode = false, preserveId = null) {
    if (!appendMode) startLoading();
    
    const targetId = preserveId || (appendMode ? null : selectedItem.value?.id);
    await loadItemsBase({ append: appendMode, preserveId: targetId });
    
    if (!appendMode) {
        if (allLoadedItems.value.length > 0) lastKnownId = allLoadedItems.value[0].id;
        
        if (!isCycling.value) {
            if (targetId) {
                const idx = allLoadedItems.value.findIndex((it) => it.id === targetId);
                globalSelectedIndex.value = idx >= 0 ? idx : (allLoadedItems.value.length > 0 ? 0 : -1);
            } else {
                selectFirst();
            }
            await syncPreviewWindow();
        }
    }
    
    stopLoading();
}

async function syncPreviewWindow() {
    if (!selectedItem.value) return;
    await preview.show(selectedItem.value.id);
}

watch(selectedItem, async (newItem) => {
    if (!newItem) return;
    await preview.show(newItem.id);
});

watch(showInlinePreview, async () => {
    if (!selectedItem.value) return;
    await preview.show(selectedItem.value.id);
});

let searchDebounceTimer = null;
watch(searchQuery, () => {
    globalSelectedIndex.value = -1;
    if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
        loadItems();
    }, 300);
});

watch(allLoadedItems, async (items) => {
    if (items.length === 0) {
        globalSelectedIndex.value = -1;
        await preview.hide();
    }
});

watch(loadMoreSentinel, (el, prev) => {
    if (!loadMoreObserver) return;
    if (prev) loadMoreObserver.unobserve(prev);
    if (el) loadMoreObserver.observe(el);
});

async function deleteItem(id) {
    const success = await deleteItemBase(id);
    if (success && globalSelectedIndex.value >= allLoadedItems.value.length) {
        globalSelectedIndex.value = Math.max(0, allLoadedItems.value.length - 1);
    }
}

async function restartApi() {
    showToast("Restarting API...", { timeout: 2000 });
    const success = await restartApiBase();
    if (success) {
        await loadItems();
        showToast("API restarted successfully", { timeout: 2000 });
    } else {
        showToast("Failed to restart API", { timeout: 4000 });
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

function handleSearchKeyDown(e) {
    if (matchesConfiguredShortcut(e)) {
        e.preventDefault();
    }
}

function handleKeyDown(e) {
    updateKeyboardState(e);
    
    const target = e.target;
    const isSearchInput = target === searchInputRef.value;
    const isEditing = (['TEXTAREA', 'INPUT'].includes(target.tagName) || target.isContentEditable) && !isSearchInput;
    
    if (isEditing && (['Enter', 'NumpadEnter'].includes(e.code) || ['ArrowUp', 'ArrowDown'].includes(e.key))) {
        return;
    }
    
    if (matchesConfiguredShortcut(e)) {
        e.preventDefault();
        if (!isCycling.value) {
            startCycling();
        } else {
            cycleNext();
        }
        return;
    }
    
    if (hasAnyShortcutModifier(e)) {
        isModifierPressed.value = true;
    }
    
    if (e.metaKey && !e.shiftKey && !e.altKey && !e.ctrlKey) {
        const key = e.key;
        
        if (key === ",") {
            e.preventDefault();
            openSettings();
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
            selectNext(); 
        } else if (e.key === "ArrowUp") { 
            e.preventDefault(); 
            selectPrev(); 
        } else if (['Enter', 'NumpadEnter'].includes(e.code)) {
            const currentItem = selectedItem.value;
            const handled = handleItemShortcuts(e, currentItem, {
                paste: (item) => pasteItemToSystem(item),
                copy: (item) => copyItemToSystem(item),
                pastePlain: (item) => pasteItemPlainText(item),
                copyPlain: (item) => copyItemPlainText(item),
                openDashboard: (item) => {
                    preview.openInDashboard(item.id);
                    hideApp();
                }
            });
            if (!handled && currentItem) {
                e.preventDefault();
                pasteItemToSystem(currentItem);
            }
        }
    }
}

function handleKeyUp(e) {
    updateKeyboardState(e);
    
    const s = configuredShortcut.value;
    const modifierReleased = 
        (s.ctrl && e.key === 'Control') ||
        (s.alt && e.key === 'Alt') ||
        (s.shift && e.key === 'Shift') ||
        (s.meta && e.key === 'Meta');
    
    if (modifierReleased) {
        isModifierPressed.value = false;
        if (isCycling.value) {
            endCycling();
        }
    }
    if (e.key === "Escape") {
        if (showDirModal.value) {
            localStorage.setItem('ignoreDirMismatch', 'true');
            showDirModal.value = false;
            loadItems();
            return;
        }
        hideApp();
    }
}

async function pasteItemToSystem(item) {
    await pasteItem(item);
    loadItems(false, item.id);
}

async function copyItemToSystem(item) {
    await copyItem(item);
}

async function pasteItemPlainText(item) {
    const success = await pasteAsPlainText(item);
    if (success) {
        loadItems(false, item.id);
    } else {
        showToast("Failed to paste as plain text", { timeout: 3000, bottom: "20px" });
    }
}

async function copyItemPlainText(item) {
    const success = await copyAsPlainText(item);
    if (success) {
        showToast("Copied as plain text", { timeout: 1500, bottom: "20px" });
    } else {
        showToast("No plain text available", { timeout: 2000, bottom: "20px" });
    }
}

async function refreshItem(id) {
    const targetId = id || selectedItem.value?.id;
    if (!targetId) return;
    await loadItems(false, targetId);
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

function cleanup() {
    if (windowResizeObserver) {
        windowResizeObserver.disconnect();
        windowResizeObserver = null;
    }
    if (loadMoreObserver) {
        loadMoreObserver.disconnect();
        loadMoreObserver = null;
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
    if (searchDebounceTimer) {
        clearTimeout(searchDebounceTimer);
        searchDebounceTimer = null;
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
    
    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("keyup", handleKeyUp);
    window.addEventListener('show-toast', handleToastEvent);

    await loadTotal();
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
            loadMore();
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
                loadMore();
            }
        };
        clipboardListRef.value.addEventListener("scroll", handleListScroll, { passive: true });
        listScrollCleanup = () => {
            clipboardListRef.value?.removeEventListener("scroll", handleListScroll);
        };
    }
});

onUnmounted(() => {
    document.removeEventListener("keydown", handleKeyDown);
    document.removeEventListener("keyup", handleKeyUp);
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

        <div class="content-area" :class="{ 'has-preview': showInlinePreview }">
            <div class="items-container">
                <div v-if="apiStatus === 'error'" class="api-error-banner">
                    <span>⚠️ API not responding</span>
                    <button @click="restartApi" class="restart-btn">Restart API</button>
                    <button @click="dismissApiError" class="dismiss-btn">×</button>
                </div>
                <div v-if="isEmpty" class="empty-state">
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
                    <div v-if="hasMore" ref="loadMoreSentinel" class="load-more-sentinel">
                        <div v-if="isLoadingMore" class="loading-more">
                            <div class="spinner"></div>
                        </div>
                    </div>
                </div>
            </div>
            <div v-if="showInlinePreview" class="inline-preview">
                <InlinePreview ref="inlinePreviewRef" :itemId="selectedItem?.id" :keyboardState="keyboardState" @refresh="refreshItem" />
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
.api-error-banner {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    border-radius: 6px;
    padding: 6px 10px;
    margin: 6px 8px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-primary);
    flex-shrink: 0;
    span { flex: 1; }
    .restart-btn {
        background: var(--accent);
        color: var(--accent-text);
        border: none;
        border-radius: 4px;
        padding: 3px 8px;
        font-size: 10px;
        cursor: pointer;
        &:hover { filter: brightness(1.1); }
    }
    .dismiss-btn {
        background: transparent;
        border: none;
        color: var(--text-secondary);
        cursor: pointer;
        font-size: 14px;
        padding: 0 4px;
        &:hover { color: var(--text-primary); }
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
