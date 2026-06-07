/**
 * 从后端响应中提取错误消息
 * 后端统一返回格式: {"error": {"type": "...", "message": "..."}}
 */
import { type ApiError, isApiError, getErrorMessage as _getErrorMessage } from '@/types/api-error'

export function extractErrorMessage(error: unknown, defaultMessage = '操作失败'): string {
  return _getErrorMessage(error, defaultMessage)
}

/**
 * 错误类型枚举
 */
export const ErrorType = {
  NETWORK_ERROR: 'network_error',
  AUTH_ERROR: 'auth_error',
  VALIDATION_ERROR: 'validation_error',
  NOT_FOUND: 'not_found',
  PROVIDER_ERROR: 'provider_error',
  QUOTA_EXCEEDED: 'quota_exceeded',
  RATE_LIMIT: 'rate_limit',
  MODEL_NOT_SUPPORTED: 'model_not_supported',
  INTERNAL_ERROR: 'internal_error',
  HTTP_ERROR: 'http_error'
} as const

export type ErrorType = typeof ErrorType[keyof typeof ErrorType]

/**
 * 从后端响应中提取错误类型
 */
export function extractErrorType(error: unknown): ErrorType | null {
  if (isApiError(error) && error.response?.data?.error?.type) {
    return error.response.data.error.type as ErrorType
  }
  return null
}

/**
 * 检查是否为特定类型的错误
 */
export function isErrorType(error: unknown, type: ErrorType): boolean {
  return extractErrorType(error) === type
}

// 重新导出类型
export type { ApiError }
export { isApiError }
