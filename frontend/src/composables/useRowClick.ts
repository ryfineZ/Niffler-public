import { ref } from 'vue'

/**
 * 处理表格行点击的 composable
 * 用于在点击行时打开详情抽屉，同时排除文本选择操作
 */
export function useRowClick() {
  // 记录 mousedown 时的位置和选中状态
  const mouseDownPos = ref<{ x: number; y: number } | null>(null)
  const hadSelectionOnMouseDown = ref(false)

  /**
   * 处理 mousedown 事件，记录位置和选中状态
   */
  function handleMouseDown(event: MouseEvent) {
    mouseDownPos.value = { x: event.clientX, y: event.clientY }
    const selection = window.getSelection()
    hadSelectionOnMouseDown.value = !!(selection && selection.toString().trim().length > 0)
  }

  /**
   * 检查是否应该触发行点击事件
   * 如果用户正在选择文本或取消选中，则返回 false
   */
  function shouldTriggerRowClick(event?: MouseEvent): boolean {
    // 如果 mousedown 时已有选中文本，说明用户可能是在取消选中
    if (hadSelectionOnMouseDown.value) {
      hadSelectionOnMouseDown.value = false
      mouseDownPos.value = null
      return false
    }

    // 如果鼠标移动超过阈值，说明用户在拖动选择
    if (event && mouseDownPos.value) {
      const dx = Math.abs(event.clientX - mouseDownPos.value.x)
      const dy = Math.abs(event.clientY - mouseDownPos.value.y)
      if (dx > 5 || dy > 5) {
        mouseDownPos.value = null
        return false
      }
    }

    mouseDownPos.value = null
    return true
  }

  /**
   * 创建一个行点击处理函数
   * @param callback 当应该触发点击时执行的回调
   */
  function createRowClickHandler<T>(callback: (item: T) => void) {
    return (event: MouseEvent, item: T) => {
      if (shouldTriggerRowClick(event)) {
        callback(item)
      }
    }
  }

  return {
    handleMouseDown,
    shouldTriggerRowClick,
    createRowClickHandler
  }
}
