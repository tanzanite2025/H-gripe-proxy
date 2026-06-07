import { Switch } from '@/components/tailwind/Switch'

interface SettingSwitchCardProps {
  title: string
  description: string
  checked: boolean
  onCheckedChange: (checked: boolean) => void
}

export function SettingSwitchCard({
  title,
  description,
  checked,
  onCheckedChange,
}: SettingSwitchCardProps) {
  return (
    <div className="flex items-center justify-between rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
      <div>
        <p className="text-sm font-medium">{title}</p>
        <p className="mt-1 text-xs text-gray-500">{description}</p>
      </div>
      <Switch checked={checked} onCheckedChange={onCheckedChange} />
    </div>
  )
}
