import { useQuery, type UseQueryOptions } from '@tanstack/react-query'

import {
  getCurrentEgressIdentity,
  type CurrentEgressIdentity,
} from '@/services/cmds/diagnostics'

export const CURRENT_EGRESS_IDENTITY_QUERY_KEY = [
  'current-egress-identity',
] as const

type CurrentEgressIdentityQueryOptions = Omit<
  UseQueryOptions<
    CurrentEgressIdentity,
    Error,
    CurrentEgressIdentity,
    typeof CURRENT_EGRESS_IDENTITY_QUERY_KEY
  >,
  'queryKey' | 'queryFn'
>

export const useCurrentEgressIdentity = (
  options: CurrentEgressIdentityQueryOptions = {},
) =>
  useQuery({
    queryKey: CURRENT_EGRESS_IDENTITY_QUERY_KEY,
    queryFn: getCurrentEgressIdentity,
    ...options,
  })
