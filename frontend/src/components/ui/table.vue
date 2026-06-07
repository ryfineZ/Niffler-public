<script setup lang="ts">
import { cn } from '@/lib/utils'
import { computed } from 'vue'

interface Props {
  class?: string
}

const props = defineProps<Props>()

const tableClass = computed(() =>
  cn('w-full caption-bottom text-sm', props.class)
)
</script>

<template>
  <div class="relative w-full overflow-auto">
    <table :class="tableClass">
      <slot />
    </table>
  </div>
</template>
