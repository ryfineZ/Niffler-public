<script setup lang="ts">
import { SelectValue as SelectValuePrimitive } from 'radix-vue'

interface Props {
  placeholder?: string
}

defineProps<Props>()
</script>

<template>
  <SelectValuePrimitive :placeholder="placeholder">
    <slot />
  </SelectValuePrimitive>
</template>
