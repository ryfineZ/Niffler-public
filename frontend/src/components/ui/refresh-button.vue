<template>
  <Button
    variant="ghost"
    size="icon"
    class="h-8 w-8"
    :disabled="loading"
    :title="title"
    @click="handleClick"
  >
    <RefreshCcw
      class="w-3.5 h-3.5"
      :class="loading ? 'animate-spin' : ''"
    />
  </Button>
</template>

<script setup lang="ts">
import { Button } from '@/components/ui'
import { RefreshCcw } from 'lucide-vue-next'

interface Props {
  loading?: boolean
  title?: string
}

interface Emits {
  (e: 'click'): void
}

withDefaults(defineProps<Props>(), {
  loading: false,
  title: '刷新'
})

const emit = defineEmits<Emits>()

function handleClick() {
  emit('click')
}
</script>
