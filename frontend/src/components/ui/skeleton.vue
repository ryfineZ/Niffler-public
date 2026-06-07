<script setup lang="ts">
import { cn } from '@/lib/utils'
import { computed } from 'vue'

interface Props {
  class?: string
}

const props = defineProps<Props>()

const skeletonClass = computed(() =>
  cn('animate-pulse rounded-md bg-muted', props.class)
)
</script>

<template>
  <div :class="skeletonClass" />
</template>
