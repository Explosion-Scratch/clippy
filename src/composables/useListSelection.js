import { ref, computed, nextTick, toValue } from "vue";

/**
 * Composable for managing list selection with keyboard navigation.
 * Handles arrow key navigation, hover selection, and scroll-into-view.
 * 
 * @param {Array|Ref<Array>} items - The list of items to navigate
 * @param {Object} options - Configuration options
 * @param {Ref<HTMLElement>} [options.listRef] - Reference to the scrollable list container
 * @param {number} [options.initialIndex=-1] - Initial selected index
 * @param {number} [options.hoverLockDuration=180] - Duration to lock hover selection after keyboard nav (ms)
 * @param {number} [options.loadMoreThreshold=5] - Items from end to trigger loadMore
 * @param {Function} [options.onLoadMore] - Callback when approaching end of list
 * @param {Function} [options.onSelect] - Callback when selection changes
 * @returns {Object} Selection state and navigation methods
 */
export function useListSelection(items, options = {}) {
    const {
        listRef = ref(null),
        initialIndex = -1,
        hoverLockDuration = 180,
        loadMoreThreshold = 5,
        onLoadMore = null,
        onSelect = null
    } = options;
    
    const selectedIndex = ref(initialIndex);
    let hoverLockUntil = 0;

    const selectedItem = computed(() => {
        const list = toValue(items);
        const idx = selectedIndex.value;
        if (!list || idx < 0 || idx >= list.length) return null;
        return list[idx];
    });

    const hasSelection = computed(() => selectedIndex.value >= 0);

    function lockHoverSelection() {
        hoverLockUntil = Date.now() + hoverLockDuration;
    }

    function scrollIntoView() {
        nextTick(() => {
            const container = toValue(listRef);
            if (!container) return;
            
            const selectedEl = container.querySelector('.clipboard-item.is-selected, [data-selected="true"]');
            if (selectedEl) {
                selectedEl.scrollIntoView({ block: 'nearest', behavior: 'auto' });
            }
        });
    }

    async function selectNext() {
        const list = toValue(items);
        if (!list || list.length === 0) return selectedIndex.value;
        
        lockHoverSelection();
        
        if (selectedIndex.value < list.length - 1) {
            selectedIndex.value++;
            scrollIntoView();
            
            if (onSelect) {
                onSelect(selectedItem.value, selectedIndex.value);
            }
            
            if (onLoadMore && selectedIndex.value >= list.length - loadMoreThreshold) {
                await onLoadMore();
            }
        }
        
        return selectedIndex.value;
    }

    function selectPrev() {
        const list = toValue(items);
        if (!list || list.length === 0) return selectedIndex.value;
        
        lockHoverSelection();
        
        if (selectedIndex.value > 0) {
            selectedIndex.value--;
            scrollIntoView();
            
            if (onSelect) {
                onSelect(selectedItem.value, selectedIndex.value);
            }
        }
        
        return selectedIndex.value;
    }

    function selectIndex(index) {
        const list = toValue(items);
        const idx = toValue(index);
        
        if (!list || idx < 0 || idx >= list.length) return;
        
        selectedIndex.value = idx;
        
        if (onSelect) {
            onSelect(selectedItem.value, selectedIndex.value);
        }
    }

    function handleMouseEnter(index) {
        if (Date.now() < hoverLockUntil) return;
        selectIndex(index);
    }

    function selectFirst() {
        const list = toValue(items);
        if (!list || list.length === 0) {
            selectedIndex.value = -1;
            return;
        }
        selectedIndex.value = 0;
        scrollIntoView();
        
        if (onSelect) {
            onSelect(selectedItem.value, selectedIndex.value);
        }
    }

    function selectLast() {
        const list = toValue(items);
        if (!list || list.length === 0) {
            selectedIndex.value = -1;
            return;
        }
        selectedIndex.value = list.length - 1;
        scrollIntoView();
        
        if (onSelect) {
            onSelect(selectedItem.value, selectedIndex.value);
        }
    }

    function reset() {
        selectedIndex.value = initialIndex;
        hoverLockUntil = 0;
        
        const container = toValue(listRef);
        if (container) {
            container.scrollTop = 0;
        }
    }

    return {
        selectedIndex,
        selectedItem,
        hasSelection,
        selectNext,
        selectPrev,
        selectIndex,
        selectFirst,
        selectLast,
        handleMouseEnter,
        scrollIntoView,
        reset
    };
}
