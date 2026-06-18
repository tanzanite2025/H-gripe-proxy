import { useAppRefreshers } from '@/providers/app-data-context'
import { deleteRuntimeRule, disableRuntimeRules } from '@/services/rule-runtime'
import type { Rule } from '@/types/mihomo'
import { cn } from '@/utils/cn'

const COLOR = [
  'text-primary dark:text-primary-dark-mode',
  'text-gray-600 dark:text-gray-400',
  'text-teal-500 dark:text-teal-400',
  'text-yellow-500 dark:text-yellow-400',
  'text-green-500 dark:text-green-400',
]

interface Props {
  value: Rule
  isShadowed?: boolean
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

const sourceLabel = (source: string) => {
  if (source.startsWith('provider:')) return source.slice(9)
  if (source === 'profile') return 'profile'
  return source
}

const sourceColor = (source: string) => {
  if (source.startsWith('provider:')) return 'bg-teal-100 text-teal-600 dark:bg-teal-900/30 dark:text-teal-400'
  if (source === 'profile') return 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'
  return 'bg-purple-100 text-purple-600 dark:bg-purple-900/30 dark:text-purple-400'
}

const RuleItem = (props: Props) => {
  const { value, isShadowed } = props
  const { refreshRules } = useAppRefreshers()
  const isDisabled = value.extra?.disabled ?? false
  const isDeleted = value.extra?.deleted ?? false
  const hitCount = value.extra?.hitCount ?? 0
  const missCount = value.extra?.missCount ?? 0
  const hitAt = value.extra?.hitAt
  const totalAttempts = hitCount + missCount
  const hitRate = totalAttempts > 0 ? ((hitCount / totalAttempts) * 100).toFixed(1) : null

  const handleToggle = async () => {
    await disableRuntimeRules({ [value.index]: !isDisabled })
    refreshRules()
  }

  const handleDelete = async () => {
    await deleteRuntimeRule(value.index)
    refreshRules()
  }

  return (
    <div
      className={cn(
        'flex items-center border-b border-divider-light px-4 py-1 text-gray-900 dark:border-divider-dark dark:text-gray-100',
        (isDisabled || isDeleted) && 'opacity-40',
        isDeleted && 'line-through',
        isShadowed && 'bg-yellow-50/50 dark:bg-yellow-900/10',
      )}
    >
      <span className="mr-4 min-w-[30px] text-center text-sm leading-8 text-gray-600 dark:text-gray-400">
        {value.index + 1}
      </span>

      <div className="select-text flex-1 min-w-0">
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

      <span className={cn('ml-2 px-1.5 py-0.5 rounded text-xs font-medium', sourceColor(value.source))}>
        {sourceLabel(value.source)}
      </span>

      {hitCount > 0 && (
        <span className="ml-2 text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap" title={`Hit: ${hitCount}, Miss: ${missCount}${hitRate ? `, Rate: ${hitRate}%` : ''}${hitAt ? `, Last: ${new Date(hitAt).toLocaleTimeString()}` : ''}`}>
          {hitCount} hits{hitRate ? ` (${hitRate}%)` : ''}
        </span>
      )}

      {!isDeleted && (
        <button
          type="button"
          onClick={handleToggle}
          className={cn(
            'ml-2 px-1.5 py-0.5 rounded text-xs font-medium transition-colors',
            isDisabled
              ? 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400'
              : 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400',
          )}
          title={isDisabled ? 'Enable rule' : 'Disable rule'}
        >
          {isDisabled ? 'OFF' : 'ON'}
        </button>
      )}

      <button
        type="button"
        onClick={handleDelete}
        className={cn(
          'ml-1 px-1.5 py-0.5 rounded text-xs font-medium transition-colors',
          isDeleted
            ? 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900/30 dark:text-yellow-400'
            : 'bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400 hover:bg-red-100 hover:text-red-600 dark:hover:bg-red-900/30 dark:hover:text-red-400',
        )}
        title={isDeleted ? 'Undelete rule' : 'Delete rule'}
      >
        {isDeleted ? 'DEL' : '×'}
      </button>
    </div>
  )
}

export default RuleItem
