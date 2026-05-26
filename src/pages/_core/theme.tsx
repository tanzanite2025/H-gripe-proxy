import getSystem from '@/utils/misc'
const OS = getSystem()

// default theme setting
export const defaultTheme = {
  primary_color: '#111827', // 深碳素黑
  secondary_color: '#FC9B76',
  primary_text: '#000000',
  secondary_text: '#3C3C4399',
  background_color: '#f8f9fb', // 精致浅冷灰
  font_family: `'Outfit', 'Inter', -apple-system, BlinkMacSystemFont, "Microsoft YaHei UI", "Microsoft YaHei", 'Segoe UI', Roboto, "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji"${
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
}
