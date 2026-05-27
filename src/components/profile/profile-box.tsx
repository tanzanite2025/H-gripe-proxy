import { cn } from '@/utils/cn'

interface ProfileBoxProps {
  selected?: boolean
  children: React.ReactNode
  className?: string
  onClick?: () => void
}

export const ProfileBox = ({
  selected = false,
  children,
  className,
  onClick,
}: ProfileBoxProps) => {
  return (
    <div
      role="button"
      aria-selected={selected}
      onClick={onClick}
      className={cn(
        'relative block cursor-pointer rounded-lg p-4 text-left',
        'bg-white dark:bg-[#282A36]',
        'text-gray-600 dark:text-gray-400',
        selected && 'border-l-4 border-blue-500 -ml-1 w-[calc(100%+4px)]',
        !selected && 'w-full',
        className,
      )}
      style={{
        boxSizing: 'border-box',
      }}
    >
      <div
        className={cn(
          selected
            ? 'text-blue-500 [&_h2]:text-blue-500'
            : '[&_h2]:text-gray-900 dark:[&_h2]:text-gray-100',
        )}
      >
        {children}
      </div>
    </div>
  )
}
