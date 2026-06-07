import { ref, type Ref } from 'vue'
import { useToast } from './useToast'
import { parseApiError } from '@/utils/errorParser'

/**
 * 异步操作通用逻辑
 *
 * 统一处理：
 * - Loading 状态管理
 * - try/catch 错误处理
 * - Toast 通知（成功/失败）
 * - 可选的成功/失败回调
 *
 * @example
 * ```typescript
 * const { loading, execute } = useAsyncAction()
 *
 * // 简单用法
 * await execute(() => api.deleteItem(id), {
 *   successMessage: '删除成功',
 * })
 *
 * // 带回调
 * await execute(() => api.createItem(data), {
 *   successMessage: '创建成功',
 *   onSuccess: (result) => {
 *     router.push(`/items/${result.id}`)
 *   },
 * })
 *
 * // 自定义错误消息
 * await execute(() => api.updateItem(id, data), {
 *   successMessage: '更新成功',
 *   errorMessage: '更新失败，请重试',
 * })
 * ```
 */
export interface UseAsyncActionOptions<T> {
  /** 成功时显示的消息 */
  successMessage?: string
  /** 成功消息的标题 */
  successTitle?: string
  /** 失败时显示的消息（如果不提供，将解析 API 错误） */
  errorMessage?: string
  /** 失败消息的标题 */
  errorTitle?: string
  /** 成功时的回调 */
  onSuccess?: (result: T) => void
  /** 失败时的回调 */
  onError?: (error: unknown) => void
  /** 是否在失败时显示 toast（默认 true） */
  showErrorToast?: boolean
  /** 是否在成功时显示 toast（默认：有 successMessage 时为 true） */
  showSuccessToast?: boolean
}

export interface UseAsyncActionReturn {
  /** 是否正在执行 */
  loading: Ref<boolean>
  /** 执行异步操作 */
  execute: <T>(
    action: () => Promise<T>,
    options?: UseAsyncActionOptions<T>
  ) => Promise<T | undefined>
}

export function useAsyncAction(): UseAsyncActionReturn {
  const loading = ref(false)
  const { success, error: showError } = useToast()

  async function execute<T>(
    action: () => Promise<T>,
    options?: UseAsyncActionOptions<T>
  ): Promise<T | undefined> {
    const {
      successMessage,
      successTitle,
      errorMessage,
      errorTitle = '错误',
      onSuccess,
      onError,
      showErrorToast = true,
      showSuccessToast,
    } = options || {}

    loading.value = true
    try {
      const result = await action()

      // 显示成功消息
      const shouldShowSuccess = showSuccessToast ?? !!successMessage
      if (shouldShowSuccess && successMessage) {
        success(successMessage, successTitle)
      }

      // 调用成功回调
      onSuccess?.(result)

      return result
    } catch (error) {
      // 解析错误消息
      const message = errorMessage || parseApiError(error, '操作失败')

      // 显示错误消息
      if (showErrorToast) {
        showError(message, errorTitle)
      }

      // 调用错误回调
      onError?.(error)

      return undefined
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    execute,
  }
}

/**
 * 创建多个独立的异步操作
 *
 * 当需要在同一个组件中跟踪多个独立的 loading 状态时使用
 *
 * @example
 * ```typescript
 * const { actions, isAnyLoading } = useMultipleAsyncActions(['save', 'delete', 'refresh'])
 *
 * // 使用各自的 loading 状态
 * await actions.save.execute(() => api.save(data), { successMessage: '保存成功' })
 * await actions.delete.execute(() => api.delete(id), { successMessage: '删除成功' })
 *
 * // 检查是否有任何操作正在进行
 * <Button :disabled="isAnyLoading">操作</Button>
 * ```
 */
export function useMultipleAsyncActions<K extends string>(
  keys: K[]
): {
  actions: Record<K, UseAsyncActionReturn>
  isAnyLoading: Ref<boolean>
} {
  const actions = {} as Record<K, UseAsyncActionReturn>
  const loadingStates: Ref<boolean>[] = []

  for (const key of keys) {
    const action = useAsyncAction()
    actions[key] = action
    loadingStates.push(action.loading)
  }

  const isAnyLoading = ref(false)

  // 使用 watchEffect 来监听所有 loading 状态
  // 这里简化处理，在每次 execute 时会自动更新
  // 如果需要响应式，可以使用 computed
  const checkAnyLoading = () => {
    isAnyLoading.value = loadingStates.some((state) => state.value)
  }

  // 包装每个 action 的 execute 以更新 isAnyLoading
  for (const key of keys) {
    const originalExecute = actions[key].execute
    actions[key].execute = async (action, options) => {
      checkAnyLoading()
      try {
        return await originalExecute(action, options)
      } finally {
        checkAnyLoading()
      }
    }
  }

  return {
    actions,
    isAnyLoading,
  }
}
