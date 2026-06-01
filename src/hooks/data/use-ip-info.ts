import { useQuery } from '@tanstack/react-query'

import { getIpInfo } from '@/services/api'

export const IP_INFO_QUERY_KEY = 'cv_ip_info_cache'
const IP_INFO_REFRESH_INTERVAL = 5 * 60 * 1000

export const useIPInfo = () =>
  useQuery({
    queryKey: [IP_INFO_QUERY_KEY],
    queryFn: getIpInfo,
    staleTime: Infinity,
    gcTime: 60 * 60 * 1000,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    refetchInterval: IP_INFO_REFRESH_INTERVAL,
    retry: 1,
    retryDelay: 30_000,
  })
