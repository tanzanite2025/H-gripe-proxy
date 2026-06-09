export interface StrategyPoolGroupRef {
  name: string
  configType: IProxyGroupConfig['type']
  displayType: string
  testUrl?: string
  hidden?: boolean
  icon?: string
}

export interface ManagedStrategyPool {
  currentProxyName: string
  groupRef: StrategyPoolGroupRef
  memberCount: number
  runtimeLoaded: boolean
}
