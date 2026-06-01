import { useTranslation } from 'react-i18next'

import { BasePage } from '@/components/base'
import { TestCard } from '@/components/home/test-card'

const WebTestPage = () => {
  const { t } = useTranslation()

  return (
    <BasePage
      full
      title={t('tests.page.title')}
      contentStyle={{ padding: 2 }}
    >
      <TestCard />
    </BasePage>
  )
}

export default WebTestPage
