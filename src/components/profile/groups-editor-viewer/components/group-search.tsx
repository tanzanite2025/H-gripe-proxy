import { BaseSearchBox } from '@/components/base'

interface GroupSearchProps {
  onSearch: (match: (name: string) => boolean) => void
}

export const GroupSearch = ({ onSearch }: GroupSearchProps) => {
  return <BaseSearchBox onSearch={onSearch} />
}
