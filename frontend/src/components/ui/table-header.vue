<script setup lang="ts">
import { cn } from '@/lib/utils'
import { computed } from 'vue'

interface Props {
  class?: string
}

const props = defineProps<Props>()

const headerClass = computed(() =>
  cn('[&_tr]:border-b', props.class)
)
</script>

<template>
  <thead :class="headerClass">
    <slot />
  </thead>
</template>
