import { Inbox } from 'lucide-react'

export const ProxyRenderEmptyState = () => {
  return (
    <div className="flex flex-col items-center justify-center py-4 pl-0">
      <Inbox className="text-2xl" />
      <span>No Proxies</span>
    </div>
  )
}
