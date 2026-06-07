import { ref, computed, type Ref } from 'vue'
import type { UsageRecord } from '../types'

export interface UseUsagePaginationOptions {
  /** 数据源记录 */
  records: Ref<UsageRecord[]>
  /** 初始页码 */
  initialPage?: number
  /** 初始每页大小 */
  initialPageSize?: number
  /** 每页大小选项 */
  pageSizeOptions?: number[]
}

export function useUsagePagination(options: UseUsagePaginationOptions) {
  const {
    records,
    initialPage = 1,
    initialPageSize = 20,
    pageSizeOptions = [10, 20, 50, 100]
  } = options

  // 分页状态
  const currentPage = ref(initialPage)
  const pageSize = ref(initialPageSize)

  // 计算总页数
  const totalPages = computed(() =>
    Math.max(1, Math.ceil(records.value.length / pageSize.value))
  )

  // 计算总记录数
  const totalRecords = computed(() => records.value.length)

  // 分页后的记录
  const paginatedRecords = computed(() => {
    const start = (currentPage.value - 1) * pageSize.value
    const end = start + pageSize.value
    return records.value.slice(start, end)
  })

  // 处理页码变化
  function changePage(page: number) {
    if (page < 1 || page > totalPages.value) return
    currentPage.value = page
  }

  // 处理每页大小变化
  function changePageSize(size: number) {
    pageSize.value = size
    currentPage.value = 1  // 重置到第一页
  }

  // 重置到第一页
  function resetPage() {
    currentPage.value = 1
  }

  // 跳转到最后一页
  function goToLastPage() {
    currentPage.value = totalPages.value
  }

  return {
    // 状态
    currentPage,
    pageSize,
    pageSizeOptions,

    // 计算属性
    totalPages,
    totalRecords,
    paginatedRecords,

    // 方法
    changePage,
    changePageSize,
    resetPage,
    goToLastPage
  }
}
