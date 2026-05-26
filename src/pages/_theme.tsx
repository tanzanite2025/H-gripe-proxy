import getSystem from '@/utils/get-system'
const OS = getSystem()

// default theme setting
export const defaultTheme = {
  primary_color: '#111827', // 深碳素黑
  secondary_color: '#FC9B76',
  primary_text: '#000000',
  secondary_text: '#3C3C4399',
  info_color: '#111827',
  error_color: '#FF3B30',
  warning_color: '#FF9500',
  success_color: '#06943D',
  background_color: '#f8f9fb', // 精致浅冷灰
  font_family: `-apple-system, BlinkMacSystemFont,"Microsoft YaHei UI", "Microsoft YaHei", Roboto, "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji"${
    OS === 'windows' ? ', twemoji mozilla' : ''
  }`,
}

// dark mode
export const defaultDarkTheme = {
  ...defaultTheme,
  primary_color: '#14b8a6', // 流光水鸭青 (极光青)
  secondary_color: '#FF9F0A',
  primary_text: '#FFFFFF',
  background_color: '#0b0c0e', // 深曜石黑
  secondary_text: '#EBEBF599',
  info_color: '#14b8a6',
  error_color: '#FF453A',
  warning_color: '#FF9F0A',
  success_color: '#30D158',
}
