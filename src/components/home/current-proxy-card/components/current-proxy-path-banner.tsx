interface CurrentProxyPathBannerProps {
  pathText: string
}

export function CurrentProxyPathBanner({
  pathText,
}: CurrentProxyPathBannerProps) {
  if (!pathText) {
    return null
  }

  return (
    <div className="mb-3 rounded-2xl border border-sky-500/20 bg-sky-500/5 px-3 py-2 text-xs text-gray-300">
      <span className="break-all">{pathText}</span>
    </div>
  )
}
