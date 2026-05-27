import { cn } from '@/utils/cn'

interface TestBoxProps extends React.HTMLAttributes<HTMLDivElement> {
  'aria-selected'?: boolean
}

export const TestBox = ({ className, 'aria-selected': selected, ...props }: TestBoxProps) => {
  return (
    <div
      className={cn(
        'relative w-full cursor-pointer rounded-xl border border-dashed p-4 text-left transition-all duration-300',
        'bg-primary/5 dark:bg-white/[0.02]',
        'border-black/[0.06] dark:border-white/[0.04]',
        'hover:border-primary hover:-translate-y-0.5',
        'hover:bg-primary/10 dark:hover:bg-white/[0.06]',
        'hover:shadow-md dark:hover:shadow-xl',
        'text-gray-600 dark:text-gray-400/65',
        selected && 'text-gray-600 dark:text-gray-400/65',
        className
      )}
      aria-selected={selected}
      {...props}
    />
  )
}
