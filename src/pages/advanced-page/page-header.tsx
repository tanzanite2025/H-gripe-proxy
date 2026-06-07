import { Button, Stack } from '@/components/tailwind'

interface AdvancedPageHeaderProps {
  saving: boolean
  onLoadRecommended: () => void
  onSave: () => void
}

export function AdvancedPageHeader({
  saving,
  onLoadRecommended,
  onSave,
}: AdvancedPageHeaderProps) {
  return (
    <Stack direction="row" spacing={1}>
      <Button variant="outlined" size="small" onClick={onLoadRecommended}>
        加载推荐配置
      </Button>
      <Button
        variant="primary"
        size="small"
        onClick={onSave}
        disabled={saving}
      >
        {saving ? '保存中...' : '保存配置'}
      </Button>
    </Stack>
  )
}
