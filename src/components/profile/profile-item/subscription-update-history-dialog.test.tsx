import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, test, vi } from 'vitest'

import {
  getSubscriptionSourceUpdateEvents,
  listSubscriptionArtifactSummaries,
} from '@/services/cmds/subscriptions'
import type {
  SubscriptionUpdateEvent,
  SubscriptionUpdateStage,
} from '@/types/subscription-update'

import { SubscriptionUpdateHistoryDialog } from './subscription-update-history-dialog'

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (
      key: string,
      options?: { defaultValue?: string; [key: string]: unknown },
    ) => {
      if (!options?.defaultValue) {
        return key
      }

      return Object.entries(options).reduce((result, [name, value]) => {
        if (name === 'defaultValue') {
          return result
        }

        return result.replaceAll(`{{${name}}}`, String(value))
      }, options.defaultValue)
    },
  }),
}))

vi.mock('@/services/cmds/subscriptions', () => ({
  getSubscriptionArtifactContent: vi.fn(),
  getSubscriptionArtifactDiagnostics: vi.fn(),
  getSubscriptionSourceUpdateEvents: vi.fn(),
  listSubscriptionArtifactSummaries: vi.fn(),
}))

vi.mock('@/services/notice-service', () => ({
  showNotice: {
    error: vi.fn(),
  },
}))

const stageCases: Array<{
  stage: SubscriptionUpdateStage
  stageLabel: string
  error: string
}> = [
  {
    stage: 'fetch_payload',
    stageLabel: 'Fetch payload',
    error: 'network request timed out',
  },
  {
    stage: 'decode_payload',
    stageLabel: 'Decode payload',
    error: 'unsupported subscription payload',
  },
  {
    stage: 'validate_runtime_candidate',
    stageLabel: 'Validate runtime candidate',
    error: 'mihomo rejected generated runtime config',
  },
  {
    stage: 'activate_runtime',
    stageLabel: 'Activate runtime',
    error: 'runtime activation failed',
  },
]

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('SubscriptionUpdateHistoryDialog', () => {
  test.each(
    stageCases,
  )('renders $stageLabel failures from structured subscription events', async ({
    stage,
    stageLabel,
    error,
  }) => {
    vi.mocked(getSubscriptionSourceUpdateEvents).mockResolvedValue([
      failedUpdateEvent(stage, error),
    ])
    vi.mocked(listSubscriptionArtifactSummaries).mockResolvedValue([])

    renderDialog()

    expect(await screen.findByText(stageLabel)).toBeInTheDocument()
    expect(screen.getByText(error)).toBeInTheDocument()
    expect(screen.getByText('failed')).toBeInTheDocument()
  })
})

function renderDialog() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  })

  return render(
    <QueryClientProvider client={queryClient}>
      <SubscriptionUpdateHistoryDialog
        open={true}
        sourceId="profile-1"
        profileName="Remote profile"
        onClose={vi.fn()}
      />
    </QueryClientProvider>,
  )
}

function failedUpdateEvent(
  stage: SubscriptionUpdateStage,
  message: string,
): SubscriptionUpdateEvent {
  return {
    kind: 'update_finished',
    source_id: 'profile-1',
    attempt_id: `attempt-${stage}`,
    trigger: 'manual',
    finished_at: 1_781_533_983_000,
    final_status: 'failed',
    stage,
    transport: 'direct',
    artifact_version: null,
    runtime_activated: false,
    active_artifact_unchanged: true,
    error: {
      message,
    },
  }
}
