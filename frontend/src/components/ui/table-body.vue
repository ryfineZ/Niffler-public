<script setup lang="ts">
import { cn } from '@/lib/utils'
import { computed } from 'vue'

interface Props {
  class?: string
}

const props = defineProps<Props>()

const bodyClass = computed(() =>
  cn('[&_tr:last-child]:border-0', props.class)
)
</script>

<template>
  <tbody :class="bodyClass">
    <slot />
  </tbody>
</template>
