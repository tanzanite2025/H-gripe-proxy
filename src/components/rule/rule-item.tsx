import { Rule } from 'tauri-plugin-mihomo-api'

import { cn } from '@/utils/cn'

const COLOR = [
  'text-primary dark:text-primary-dark-mode',
  'text-gray-600 dark:text-gray-400',
  'text-blue-500 dark:text-blue-400',
  'text-yellow-500 dark:text-yellow-400',
  'text-green-500 dark:text-green-400',
]

interface Props {
  value: Rule & { lineNo: number }
}

const parseColor = (text: string) => {
  if (text === 'REJECT' || text === 'REJECT-DROP') return 'text-red-500 dark:text-red-400'
  if (text === 'DIRECT') return 'text-gray-900 dark:text-gray-100'

  let sum = 0
  for (let i = 0; i < text.length; i++) {
    sum += text.charCodeAt(i)
  }
  return COLOR[sum % COLOR.length]
}

const RuleItem = (props: Props) => {
  const { value } = props

  return (
    <div className="flex border-b border-divider-light px-4 py-1 text-gray-900 dark:border-divider-dark dark:text-gray-100">
      <span className="mr-4 min-w-[30px] text-center text-sm leading-8 text-gray-600 dark:text-gray-400">
        {value.lineNo}
      </span>

      <div className="select-text">
        <h6 className="text-base font-medium text-gray-900 dark:text-gray-100">
          {value.payload || '-'}
        </h6>

        <span className="mr-6 inline-block min-w-[120px] text-sm text-gray-600 dark:text-gray-400">
          {value.type}
        </span>

        <span className={cn('text-sm', parseColor(value.proxy))}>
          {value.proxy}
        </span>
      </div>
    </div>
  )
}

export default RuleItem
