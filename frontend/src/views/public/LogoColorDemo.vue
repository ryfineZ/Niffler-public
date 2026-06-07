<template>
  <div class="min-h-screen bg-[#fafaf7] dark:bg-[#191714] p-8">
    <div class="max-w-7xl mx-auto">
      <h1 class="text-3xl font-bold text-center mb-2 text-[#191919] dark:text-white">
        Logo 颜色方案对比
      </h1>
      <p class="text-center text-[#666663] dark:text-gray-400 mb-8">
        点击任意方案可以放大预览
      </p>

      <!-- Color schemes grid -->
      <div class="grid grid-cols-2 md:grid-cols-3 gap-6">
        <div
          v-for="(scheme, index) in colorSchemes"
          :key="index"
          class="relative bg-white dark:bg-[#262624] rounded-2xl p-6 border border-[#e5e4df] dark:border-[rgba(227,224,211,0.16)] cursor-pointer transition-all hover:shadow-lg hover:scale-[1.02]"
          :class="{ 'ring-2 ring-primary': selectedScheme === index }"
          @click="selectScheme(index)"
        >
          <!-- Scheme name badge -->
          <div
            class="absolute top-3 left-3 px-2 py-1 rounded-full text-xs font-medium"
            :style="{ backgroundColor: scheme.primary + '20', color: scheme.primary }"
          >
            {{ scheme.name }}
          </div>

          <!-- Logo preview -->
          <div class="flex items-center justify-center h-48 mb-4">
            <svg
              viewBox="0 0 800 800"
              class="w-40 h-40"
              xmlns="http://www.w3.org/2000/svg"
            >
              <defs>
                <linearGradient
                  :id="`gradient-${index}`"
                  x1="0%"
                  y1="0%"
                  x2="100%"
                  y2="100%"
                >
                  <stop
                    offset="0%"
                    :stop-color="scheme.primary"
                  />
                  <stop
                    offset="50%"
                    :stop-color="scheme.secondary"
                  />
                  <stop
                    offset="100%"
                    :stop-color="scheme.primary"
                  />
                </linearGradient>
              </defs>

              <!-- Fill -->
              <path
                :d="fullPath"
                :fill="`url(#gradient-${index})`"
                fill-rule="evenodd"
                opacity="0.7"
              />

              <!-- Lines -->
              <path
                v-for="(path, pathIndex) in linePaths"
                :key="pathIndex"
                :d="path"
                fill="none"
                :stroke="scheme.primary"
                stroke-width="3.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>

          <!-- Color swatches -->
          <div class="flex items-center justify-center gap-3">
            <div class="flex flex-col items-center">
              <div
                class="w-8 h-8 rounded-full border-2 border-white shadow"
                :style="{ backgroundColor: scheme.primary }"
              />
              <span class="text-xs text-[#666663] dark:text-gray-400 mt-1">{{ scheme.primary }}</span>
            </div>
            <div class="flex flex-col items-center">
              <div
                class="w-8 h-8 rounded-full border-2 border-white shadow"
                :style="{ backgroundColor: scheme.secondary }"
              />
              <span class="text-xs text-[#666663] dark:text-gray-400 mt-1">{{ scheme.secondary }}</span>
            </div>
          </div>

          <!-- Description -->
          <p class="text-center text-sm text-[#666663] dark:text-gray-400 mt-3">
            {{ scheme.description }}
          </p>
        </div>
      </div>

      <!-- Large preview modal -->
      <Teleport to="body">
        <div
          v-if="showPreview"
          class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
          @click="showPreview = false"
        >
          <div
            class="bg-white dark:bg-[#262624] rounded-3xl p-8 max-w-lg w-full mx-4 shadow-2xl"
            @click.stop
          >
            <div class="flex items-center justify-between mb-6">
              <h2 class="text-xl font-bold text-[#191919] dark:text-white">
                {{ colorSchemes[selectedScheme].name }}
              </h2>
              <button
                class="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
                @click="showPreview = false"
              >
                <svg
                  class="w-5 h-5"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="2"
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>

            <!-- Animated logo -->
            <div class="flex items-center justify-center py-8">
              <AetherLineByLineLogo
                :key="selectedScheme"
                :size="300"
                :line-delay="120"
                :stroke-duration="2000"
                :color-duration="1200"
                :auto-start="true"
                :loop="true"
                :loop-pause="300"
                :stroke-width="3.5"
                :outline-color="colorSchemes[selectedScheme].primary"
              />
            </div>

            <!-- Color info -->
            <div class="flex items-center justify-center gap-6 mt-4">
              <div class="flex items-center gap-2">
                <div
                  class="w-6 h-6 rounded-full border-2 border-white shadow"
                  :style="{ backgroundColor: colorSchemes[selectedScheme].primary }"
                />
                <span class="text-sm font-mono text-[#666663] dark:text-gray-400">
                  {{ colorSchemes[selectedScheme].primary }}
                </span>
              </div>
              <div class="flex items-center gap-2">
                <div
                  class="w-6 h-6 rounded-full border-2 border-white shadow"
                  :style="{ backgroundColor: colorSchemes[selectedScheme].secondary }"
                />
                <span class="text-sm font-mono text-[#666663] dark:text-gray-400">
                  {{ colorSchemes[selectedScheme].secondary }}
                </span>
              </div>
            </div>

            <!-- Apply button -->
            <div class="mt-6 text-center">
              <button
                class="px-6 py-2 bg-primary text-white rounded-xl font-medium hover:bg-primary/90 transition"
                @click="applyScheme"
              >
                应用此方案
              </button>
            </div>
          </div>
        </div>
      </Teleport>

      <!-- Back button -->
      <div class="mt-8 text-center">
        <RouterLink
          to="/"
          class="inline-flex items-center gap-2 px-4 py-2 text-[#666663] dark:text-gray-400 hover:text-[#191919] dark:hover:text-white transition"
        >
          <svg
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M15 19l-7-7 7-7"
            />
          </svg>
          返回首页
        </RouterLink>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink } from 'vue-router'
import AetherLineByLineLogo from '@/components/AetherLineByLineLogo.vue'
import { AETHER_LINE_PATHS, AETHER_FULL_PATH } from '@/constants/logoPaths'

const linePaths = AETHER_LINE_PATHS
const fullPath = AETHER_FULL_PATH

const colorSchemes = [
  {
    name: '当前配色 - 暖橙',
    primary: '#cc785c',
    secondary: '#e8a882',
    description: '温暖的赤陶色，亲和力强'
  },
  {
    name: '深金色调',
    primary: '#b08d57',
    secondary: '#d4b896',
    description: '古铜金色，更加沉稳大气'
  },
  {
    name: '玫瑰红调',
    primary: '#a4636c',
    secondary: '#d4a5aa',
    description: '优雅的玫瑰红，细腻温婉'
  },
  {
    name: '青铜绿调',
    primary: '#7a8c70',
    secondary: '#a8b8a0',
    description: '自然青铜色，清新科技感'
  },
  {
    name: '深紫藤调',
    primary: '#7d6b8a',
    secondary: '#b5a8c2',
    description: '紫藤色调，神秘优雅'
  },
  {
    name: '黑金商务',
    primary: '#3d3833',
    secondary: '#8b7355',
    description: '炭黑配金色，高端商务风'
  },
  {
    name: '海蓝科技',
    primary: '#4a7c8c',
    secondary: '#8ab8c8',
    description: '深海蓝色，科技专业感'
  },
  {
    name: '焦糖棕调',
    primary: '#8b6b4a',
    secondary: '#c4a882',
    description: '焦糖棕色，复古温暖'
  },
  {
    name: '石墨灰调',
    primary: '#5a5a5a',
    secondary: '#9a9a9a',
    description: '中性石墨色，简约现代'
  }
]

const selectedScheme = ref(0)
const showPreview = ref(false)

const selectScheme = (index: number) => {
  selectedScheme.value = index
  showPreview.value = true
}

const applyScheme = () => {
  const scheme = colorSchemes[selectedScheme.value]
  alert(`应用方案: ${scheme.name}\n\n请将以下颜色值更新到代码中:\n主色: ${scheme.primary}\n过渡色: ${scheme.secondary}\n\n需要修改的文件:\n1. AetherLineByLineLogo.vue - gradient 颜色\n2. Home.vue - outlineColor 属性`)
  showPreview.value = false
}
</script>
