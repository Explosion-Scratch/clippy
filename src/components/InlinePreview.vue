<script setup>
import PreviewPane from "./PreviewPane.vue";

defineProps({
    itemId: String
});

const emit = defineEmits(["refresh"]);
</script>

<template>
    <div class="inline-preview-inner">
        <PreviewPane 
            :item-id="itemId" 
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
