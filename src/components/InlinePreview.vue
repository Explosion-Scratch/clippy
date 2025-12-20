<script setup>
import { ref } from "vue";
import PreviewPane from "./PreviewPane.vue";

defineProps({
    itemId: String,
    keyboardState: {
        type: Object,
        default: () => ({ currentlyPressed: [], itemShortcuts: [] })
    }
});

const emit = defineEmits(["refresh"]);
const previewPaneRef = ref(null);

defineExpose({
    resetState: () => previewPaneRef.value?.resetState()
});
</script>

<template>
    <div class="inline-preview-inner">
        <PreviewPane 
            ref="previewPaneRef"
            :item-id="itemId" 
            :keyboard-state="keyboardState"
            :is-inline="true"
            @refresh="(newId) => emit('refresh', newId)" 
        />
    </div>
</template>

<style scoped>
.inline-preview-inner {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
}
</style>

