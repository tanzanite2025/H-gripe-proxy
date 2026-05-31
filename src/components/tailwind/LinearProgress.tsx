import { cn } from '@/utils/cn'

interface LinearProgressProps {
  variant?: 'determinate' | 'indeterminate'
  value?: number
  color?: 'primary' | 'success' | 'warning' | 'error'
  className?: string
  style?: React.CSSProperties
}

const colorMap: Record<string, string> = {
  primary: 'bg-primary',
  success: 'bg-green-500',
  warning: 'bg-yellow-500',
  error: 'bg-red-500',
}

export const LinearProgress = ({
  variant = 'indeterminate',
  value = 0,
  color = 'primary',
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
          colorMap[color] || 'bg-primary',
          'h-full transition-all duration-300',
          variant === 'indeterminate' && 'animate-pulse',
        )}
        style={{
          width: variant === 'determinate' ? `${value}%` : '100%',
        }}
      />
    </div>
  )
}
