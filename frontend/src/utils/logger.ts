/**
 * 统一日志工具
 * 生产环境不输出日志,开发环境按级别输出
 */

import { LogLevel, isDev } from '@/config/constants'

class Logger {
  private isDevelopment: boolean

  constructor() {
    this.isDevelopment = isDev
  }

  /**
   * 格式化日志消息
   */
  private formatMessage(level: LogLevel, message: string, context?: unknown): string {
    const timestamp = new Date().toISOString()
    const contextStr = context ? ` | ${JSON.stringify(context)}` : ''
    return `[${timestamp}] [${level}] ${message}${contextStr}`
  }

  /**
   * 调试日志 - 仅开发环境
   */
  debug(message: string, context?: unknown): void {
    if (this.isDevelopment) {
      console.debug(this.formatMessage(LogLevel.DEBUG, message, context))
    }
  }

  /**
   * 信息日志 - 仅开发环境
   */
  info(message: string, context?: unknown): void {
    if (this.isDevelopment) {
      console.info(this.formatMessage(LogLevel.INFO, message, context))
    }
  }

  /**
   * 警告日志 - 仅开发环境
   */
  warn(message: string, context?: unknown): void {
    if (this.isDevelopment) {
      console.warn(this.formatMessage(LogLevel.WARN, message, context))
    }
  }

  /**
   * 错误日志 - 始终输出(但在生产环境可以发送到监控服务)
   */
  error(message: string, error?: unknown): void {
    const errorContext = error instanceof Error
      ? { message: error.message, stack: error.stack }
      : error

    if (this.isDevelopment) {
      console.error(this.formatMessage(LogLevel.ERROR, message, errorContext))
    } else {
      // 生产环境:可以在这里发送到错误监控服务(如 Sentry)
      // 目前只记录到 console.error,不暴露详细信息
      console.error(`[ERROR] ${message}`)
    }
  }

  /**
   * 网络请求日志
   */
  http(method: string, url: string, status?: number, duration?: number): void {
    if (this.isDevelopment) {
      const statusText = status ? `[${status}]` : ''
      const durationText = duration ? `(${duration}ms)` : ''
      this.info(`HTTP ${method} ${url} ${statusText} ${durationText}`)
    }
  }

  /**
   * 性能日志
   */
  performance(label: string, duration: number): void {
    if (this.isDevelopment) {
      this.info(`Performance: ${label} took ${duration}ms`)
    }
  }
}

// 导出单例
export const logger = new Logger()

// 便捷方法
export const log = {
  debug: (message: string, context?: unknown) => logger.debug(message, context),
  info: (message: string, context?: unknown) => logger.info(message, context),
  warn: (message: string, context?: unknown) => logger.warn(message, context),
  error: (message: string, error?: unknown) => logger.error(message, error),
  http: (method: string, url: string, status?: number, duration?: number) =>
    logger.http(method, url, status, duration),
  performance: (label: string, duration: number) => logger.performance(label, duration),
}
