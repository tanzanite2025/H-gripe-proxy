import { useLocalStorage } from 'foxact/use-local-storage'

const defaultClashLog: IClashLog = {
  enable: false,
  logLevel: 'info',
  logFilter: 'all',
  logOrder: 'asc',
}

export const useClashLog = () =>
  useLocalStorage<IClashLog>('clash-log', defaultClashLog, {
    serializer: JSON.stringify,
    deserializer: JSON.parse,
  })
