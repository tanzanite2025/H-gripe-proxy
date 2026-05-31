import { cn } from '@/utils/cn'

interface ProfileBoxProps {
  selected?: boolean
  children: React.ReactNode
  className?: string
  onClick?: React.MouseEventHandler<HTMLDivElement>
  onDoubleClick?: React.MouseEventHandler<HTMLDivElement>
  onContextMenu?: React.MouseEventHandler<HTMLDivElement>
}

export const ProfileBox = ({
  selected = false,
  children,
  className,
  onClick,
  onDoubleClick,
  onContextMenu,
}: ProfileBoxProps) => {
  return (
    <div
      role="button"
      aria-selected={selected}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      onContextMenu={onContextMenu}
      className={cn(
        'relative block cursor-pointer rounded-lg p-4 text-left bg-card text-text-secondary',
        selected && 'border-l-4 border-teal-500 -ml-1 w-[calc(100%+4px)]',
        !selected && 'w-full',
        className,
      )}
    >
      <div
        className={cn(
          selected
            ? 'text-teal-500 [&_h2]:text-teal-500'
            : '[&_h2]:text-text-primary',
        )}
      >
        {children}
      </div>
    </div>
  )
}
