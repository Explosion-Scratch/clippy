import { ref, computed, toValue } from "vue";

/**
 * Composable for cycling through a list of items.
 * Inspired by VueUse's useCycleList pattern.
 * 
 * @param {Array|Ref<Array>} list - The list to cycle through (can be ref or plain array)
 * @param {Object} options - Configuration options
 * @param {number} [options.initialIndex=0] - Starting index
 * @returns {Object} Cycling state and control methods
 */
export function useCycleList(list, options = {}) {
    const { initialIndex = 0 } = options;
    
    const activeIndex = ref(initialIndex);
    const isCycling = ref(false);
    
    const state = computed(() => {
        const items = toValue(list);
        if (!items || items.length === 0) return null;
        const idx = activeIndex.value;
        if (idx < 0 || idx >= items.length) return null;
        return items[idx];
    });
    
    const listLength = computed(() => {
        const items = toValue(list);
        return items?.length || 0;
    });

    function next() {
        const len = listLength.value;
        if (len === 0) return state.value;
        
        activeIndex.value = (activeIndex.value + 1) % len;
        return state.value;
    }

    function prev() {
        const len = listLength.value;
        if (len === 0) return state.value;
        
        activeIndex.value = activeIndex.value === 0 
            ? len - 1 
            : activeIndex.value - 1;
        return state.value;
    }

    function go(index) {
        const len = listLength.value;
        if (len === 0) return state.value;
        
        const idx = toValue(index);
        if (idx >= 0 && idx < len) {
            activeIndex.value = idx;
        }
        return state.value;
    }

    function start(startIndex = 1) {
        const len = listLength.value;
        if (len === 0) return null;
        
        isCycling.value = true;
        activeIndex.value = Math.min(toValue(startIndex), len - 1);
        return state.value;
    }

    function stop() {
        isCycling.value = false;
        return state.value;
    }

    function reset() {
        activeIndex.value = initialIndex;
        isCycling.value = false;
    }

    return {
        state,
        activeIndex,
        isCycling,
        listLength,
        next,
        prev,
        go,
        start,
        stop,
        reset
    };
}

/**
 * Specialized cycling composable for clipboard manager's Ctrl+P cycling mode.
 * Extends useCycleList with clipboard-specific behavior.
 * 
 * @param {Array|Ref<Array>} items - Clipboard items to cycle through
 * @param {Object} options - Configuration options  
 * @param {Function} [options.onCycleEnd] - Callback when cycling ends (receives selected item)
 * @param {Function} [options.onCycleStart] - Callback when cycling starts
 * @param {Function} [options.onSelect] - Callback when selection changes
 * @returns {Object} Cycling state and methods
 */
export function useClipboardCycling(items, options = {}) {
    const { 
        onCycleEnd = null,
        onCycleStart = null,
        onSelect = null
    } = options;
    
    const {
        state,
        activeIndex,
        isCycling,
        listLength,
        next,
        reset: baseReset
    } = useCycleList(items, { initialIndex: -1 });

    async function startCycling() {
        const itemList = toValue(items);
        if (!itemList || itemList.length === 0) return null;
        
        isCycling.value = true;
        activeIndex.value = itemList.length > 1 ? 1 : 0;
        
        if (onCycleStart) {
            onCycleStart(state.value, activeIndex.value);
        }
        if (onSelect) {
            onSelect(state.value, activeIndex.value);
        }
        
        return activeIndex.value;
    }

    function cycleNext() {
        if (!isCycling.value) return activeIndex.value;
        
        next();
        
        if (onSelect) {
            onSelect(state.value, activeIndex.value);
        }
        
        return activeIndex.value;
    }

    async function endCycling() {
        if (!isCycling.value) return null;
        
        const selectedItem = state.value;
        const selectedIndex = activeIndex.value;
        
        isCycling.value = false;
        
        if (onCycleEnd && selectedItem) {
            await onCycleEnd(selectedItem, selectedIndex);
        }
        
        return selectedItem;
    }

    function cancelCycling() {
        isCycling.value = false;
        activeIndex.value = -1;
    }

    function reset() {
        baseReset();
        activeIndex.value = -1;
    }

    return {
        state,
        activeIndex,
        isCycling,
        listLength,
        startCycling,
        cycleNext,
        endCycling,
        cancelCycling,
        reset
    };
}
