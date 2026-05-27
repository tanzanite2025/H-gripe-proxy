import { cn } from '@/utils/cn'

interface LinearProgressProps {
  variant?: 'determinate' | 'indeterminate'
  value?: number
  className?: string
  style?: React.CSSProperties
}

export const LinearProgress = ({
  variant = 'indeterminate',
  value = 0,
  className,
  style,
}: LinearProgressProps) => {
  return (
    <div
      className={cn(
        'relative h-1 w-full overflow-hidden bg-primary/20',
        className,
      )}
      style={style}
    >
      <div
        className={cn(
          'h-full bg-primary transition-all duration-300',
          variant === 'indeterminate' && 'animate-pulse',
        )}
        style={{
          width: variant === 'determinate' ? `${value}%` : '100%',
        }}
      />
    </div>
  )
}
