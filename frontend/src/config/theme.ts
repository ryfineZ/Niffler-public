/**
 * 主题配置系统
 *
 * 基于 shadcn/ui 和项目自定义颜色体系
 * 提供统一的主题变量、间距、圆角、阴影等配置
 */

/**
 * 颜色系统
 *
 * 项目使用书本纸张主题色:
 * - book-cloth: 书籍封面布料色 (#cc785c / #d4a27f)
 * - kraft: 牛皮纸色 (#b97847 / #c9a26f)
 * - manilla: 马尼拉纸色 (#e8ddc5 / #d4c5a9)
 * - cloud: 云白色 (#f5f3ed / #2a2723)
 */
export const themeColors = {
  // 主色调
  primary: {
    light: '#cc785c',      // book-cloth light
    dark: '#d4a27f',       // book-cloth dark
    hover: {
      light: '#b86d52',
      dark: '#c29470'
    }
  },

  // 次要色调
  secondary: {
    light: '#b97847',      // kraft light
    dark: '#c9a26f',       // kraft dark
    hover: {
      light: '#a56a3f',
      dark: '#b8915e'
    }
  },

  // 强调色
  accent: {
    light: '#e8ddc5',      // manilla light
    dark: '#d4c5a9',       // manilla dark
  },

  // 背景色
  background: {
    light: {
      base: '#fafaf7',     // cloud light
      elevated: '#ffffff',
      muted: '#f5f3ed'
    },
    dark: {
      base: '#191714',     // cloud dark
      elevated: '#262624',
      muted: '#2a2723'
    }
  },

  // 边框色
  border: {
    light: {
      default: '#e5e4df',
      hover: 'rgba(204, 120, 92, 0.3)',
      focus: 'rgba(204, 120, 92, 0.4)'
    },
    dark: {
      default: 'rgba(227, 224, 211, 0.12)',
      hover: 'rgba(212, 162, 127, 0.4)',
      focus: 'rgba(212, 162, 127, 0.5)'
    }
  },

  // 状态色
  status: {
    success: {
      light: '#10b981',
      dark: '#34d399'
    },
    warning: {
      light: '#f59e0b',
      dark: '#fbbf24'
    },
    error: {
      light: '#ef4444',
      dark: '#f87171'
    },
    info: {
      light: '#3b82f6',
      dark: '#60a5fa'
    }
  }
} as const

/**
 * 间距系统
 *
 * 基于 8px 网格系统
 */
export const spacing = {
  // 基础间距
  xs: '4px',
  sm: '8px',
  md: '12px',
  lg: '16px',
  xl: '24px',
  '2xl': '32px',
  '3xl': '48px',
  '4xl': '64px',

  // 页面布局间距
  page: {
    padding: '24px',
    maxWidth: '1400px'
  },

  // 区块间距
  section: {
    gap: '32px',
    padding: '24px'
  },

  // 卡片间距
  card: {
    padding: '20px',
    gap: '16px'
  },

  // 表单间距
  form: {
    fieldGap: '16px',
    labelGap: '8px'
  }
} as const

/**
 * 圆角系统
 */
export const radius = {
  none: '0',
  sm: '6px',
  md: '8px',
  lg: '12px',
  xl: '16px',
  '2xl': '20px',
  full: '9999px',

  // 组件圆角
  button: '8px',
  card: '12px',
  input: '8px',
  dialog: '16px'
} as const

/**
 * 阴影系统
 */
export const shadows = {
  none: 'none',
  sm: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
  md: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
  lg: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
  xl: '0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)',
  '2xl': '0 25px 50px -12px rgba(0, 0, 0, 0.25)',

  // 组件阴影
  card: {
    light: '0 1px 3px rgba(0, 0, 0, 0.1)',
    dark: '0 1px 3px rgba(0, 0, 0, 0.3)'
  },
  elevated: {
    light: '0 4px 6px rgba(0, 0, 0, 0.1)',
    dark: '0 4px 6px rgba(0, 0, 0, 0.4)'
  },
  button: {
    light: '0 1px 2px rgba(0, 0, 0, 0.05)',
    dark: '0 1px 2px rgba(0, 0, 0, 0.2)'
  }
} as const

/**
 * 字体系统
 */
export const typography = {
  // 字体家族
  fontFamily: {
    sans: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif',
    mono: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace'
  },

  // 字体大小
  fontSize: {
    xs: '0.75rem',      // 12px
    sm: '0.875rem',     // 14px
    base: '1rem',       // 16px
    lg: '1.125rem',     // 18px
    xl: '1.25rem',      // 20px
    '2xl': '1.5rem',    // 24px
    '3xl': '1.875rem',  // 30px
    '4xl': '2.25rem',   // 36px
    '5xl': '3rem'       // 48px
  },

  // 字重
  fontWeight: {
    normal: '400',
    medium: '500',
    semibold: '600',
    bold: '700'
  },

  // 行高
  lineHeight: {
    tight: '1.25',
    normal: '1.5',
    relaxed: '1.75',
    loose: '2'
  }
} as const

/**
 * 动画系统
 */
export const animations = {
  // 过渡时间
  duration: {
    fast: '150ms',
    normal: '200ms',
    slow: '300ms',
    slower: '500ms'
  },

  // 缓动函数
  easing: {
    default: 'cubic-bezier(0.4, 0, 0.2, 1)',
    in: 'cubic-bezier(0.4, 0, 1, 1)',
    out: 'cubic-bezier(0, 0, 0.2, 1)',
    inOut: 'cubic-bezier(0.4, 0, 0.2, 1)'
  }
} as const

/**
 * 断点系统
 */
export const breakpoints = {
  sm: '640px',
  md: '768px',
  lg: '1024px',
  xl: '1280px',
  '2xl': '1536px'
} as const

/**
 * Z-index 层级系统
 */
export const zIndex = {
  base: 0,
  dropdown: 1000,
  sticky: 1020,
  fixed: 1030,
  modalBackdrop: 1040,
  modal: 1050,
  popover: 1060,
  tooltip: 1070
} as const

/**
 * 组件默认配置
 */
export const componentDefaults = {
  button: {
    height: {
      sm: '32px',
      md: '40px',
      lg: '44px'
    },
    padding: {
      sm: '8px 12px',
      md: '10px 16px',
      lg: '12px 20px'
    }
  },

  input: {
    height: {
      sm: '32px',
      md: '40px',
      lg: '44px'
    }
  },

  card: {
    padding: spacing.card.padding,
    borderRadius: radius.card
  },

  dialog: {
    sizes: {
      sm: '400px',
      md: '600px',
      lg: '800px',
      xl: '1000px'
    }
  },

  table: {
    rowHeight: '52px',
    headerHeight: '48px'
  }
} as const

/**
 * 完整主题配置
 */
export const theme = {
  colors: themeColors,
  spacing,
  radius,
  shadows,
  typography,
  animations,
  breakpoints,
  zIndex,
  components: componentDefaults
} as const

/**
 * 主题类型
 */
export type Theme = typeof theme
export type ThemeColors = typeof themeColors
export type Spacing = typeof spacing
export type Radius = typeof radius
export type Shadows = typeof shadows
export type Typography = typeof typography

/**
 * 导出默认主题
 */
export default theme
