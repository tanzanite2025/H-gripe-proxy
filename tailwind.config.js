/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{ts,tsx,html}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // 主色调
        primary: {
          DEFAULT: '#111827', // 浅色模式：深碳素黑
          dark: '#0f172a',
          light: '#1e293b',
        },
        'primary-dark-mode': {
          DEFAULT: '#14b8a6', // 深色模式：流光水鸭青
          dark: '#0d9488',
          light: '#2dd4bf',
        },
        // 次要色
        secondary: {
          DEFAULT: '#FC9B76',
          dark: '#FF9F0A',
        },
        // 背景色
        background: {
          light: '#f8f9fb', // 精致浅冷灰
          dark: '#0b0c0e', // 深曜石黑
        },
        // 卡片背景
        card: {
          light: '#ffffff',
          dark: '#16181d', // 钛金黑
        },
        // 文本颜色
        text: {
          primary: {
            light: '#000000',
            dark: '#FFFFFF',
          },
          secondary: {
            light: '#3C3C4399',
            dark: '#EBEBF599',
          },
        },
        // 分隔线
        divider: {
          light: 'rgba(0, 0, 0, 0.06)',
          dark: 'rgba(255, 255, 255, 0.04)',
        },
      },
      fontFamily: {
        sans: [
          'Outfit',
          'Inter',
          '-apple-system',
          'BlinkMacSystemFont',
          'Microsoft YaHei UI',
          'Microsoft YaHei',
          'Segoe UI',
          'Roboto',
          'Helvetica Neue',
          'Arial',
          'sans-serif',
          'Apple Color Emoji',
        ],
      },
      borderRadius: {
        card: '32px',
        button: '9999px',
        input: '16px',
        dialog: '32px',
      },
      boxShadow: {
        card: '0 2px 8px -2px rgba(0, 0, 0, 0.08)',
        'card-hover': '0 4px 12px -2px rgba(0, 0, 0, 0.12)',
        button: '0 2px 8px -2px rgba(var(--primary-rgb), 0.3)',
        dialog: '0 25px 50px -12px rgba(0, 0, 0, 0.15)',
        'dialog-dark': '0 25px 50px -12px rgba(0, 0, 0, 0.5)',
      },
      animation: {
        'fade-in': 'fadeIn 0.5s ease-in-out',
        'slide-up': 'slideUp 0.3s ease-out',
        'slide-down': 'slideDown 0.3s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { transform: 'translateY(10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
        slideDown: {
          '0%': { transform: 'translateY(-10px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
      },
      transitionTimingFunction: {
        'smooth': 'cubic-bezier(0.16, 1, 0.3, 1)',
      },
    },
  },
  plugins: [],
}
