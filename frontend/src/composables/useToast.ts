import { ref } from 'vue'
import { TOAST_CONFIG } from '@/config/constants'

export type ToastVariant = 'success' | 'error' | 'warning' | 'info'

export interface Toast {
  id: string
  title?: string
  message?: string
  variant?: ToastVariant
  duration?: number
}

const toasts = ref<Toast[]>([])

export function useToast() {
  function showToast(options: Omit<Toast, 'id'>) {
    const toast: Toast = {
      id: Date.now().toString(),
      variant: 'info',
      duration: 5000,
      ...options
    }


    toasts.value.push(toast)

    // 注释掉这里的 setTimeout，因为现在由组件自己处理
    // if (toast.duration && toast.duration > 0) {
    //   setTimeout(() => {
    //     removeToast(toast.id)
    //   }, toast.duration)
    // }

    return toast.id
  }

  function removeToast(id: string) {
    const index = toasts.value.findIndex(t => t.id === id)
    if (index > -1) {
      toasts.value.splice(index, 1)
    }
  }

  function success(message: string, title?: string) {
    return showToast({ message, title, variant: 'success', duration: TOAST_CONFIG.SUCCESS_DURATION })
  }

  function error(message: string, title?: string) {
    return showToast({ message, title, variant: 'error', duration: TOAST_CONFIG.ERROR_DURATION })
  }

  function warning(message: string, title?: string) {
    return showToast({ message, title, variant: 'warning', duration: TOAST_CONFIG.WARNING_DURATION })
  }

  function info(message: string, title?: string) {
    return showToast({ message, title, variant: 'info', duration: TOAST_CONFIG.INFO_DURATION })
  }

  function clearAll() {
    toasts.value = []
  }

  return {
    toasts,
    showToast,
    removeToast,
    success,
    error,
    warning,
    info,
    clearAll
  }
}