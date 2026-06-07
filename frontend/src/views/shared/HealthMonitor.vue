<template>
  <div class="space-y-6 pb-8">
    <HealthMonitorCard
      title="健康监控"
      :is-admin="isAdminPage"
      :show-provider-info="isAdminPage"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import HealthMonitorCard from '@/features/providers/components/HealthMonitorCard.vue'

const route = useRoute()
const isAdminPage = computed(() => route.path.startsWith('/admin'))
</script>
