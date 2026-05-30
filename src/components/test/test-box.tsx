import { cn } from '@/utils/cn'

interface TestBoxProps extends React.HTMLAttributes<HTMLDivElement> {
  'aria-selected'?: boolean
}

export const TestBox = ({ className, 'aria-selected': selected, ...props }: TestBoxProps) => {
  return (
    <div
      className={cn(
        'relative w-full cursor-pointer rounded-xl border border-dashed p-4 text-left transition-all duration-300',
        'bg-[#16181d]',
        'border-divider',
        'hover:border-primary hover:-translate-y-0.5',
        'hover:bg-primary/10',
        'hover:shadow-md',
        'text-text-secondary',
        selected && 'text-text-secondary',
        className
      )}
      aria-selected={selected}
      {...props}
    />
  )
}
