<template>
  <div
    v-show="isActive"
    :class="contentClass"
  >
    <slot />
  </div>
</template>

<script setup lang="ts">
import { computed, inject, type Ref } from 'vue'
import { cn } from '@/lib/utils'

interface Props {
  value: string
  class?: string
}

const props = defineProps<Props>()

const activeTab = inject<Ref<string>>('activeTab')

const isActive = computed(() => activeTab?.value === props.value)

const contentClass = computed(() => {
  return cn(
    'mt-2 ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
    props.class
  )
})
</script>
