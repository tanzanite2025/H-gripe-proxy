export type ProxyCardActionVariant = 'default' | 'compact'

export const ACTION_VARIANT_CLASS: Record<
  ProxyCardActionVariant,
  {
    wrapper: string
    configure: string
    loading: string
    check: string
    delay: string
    selectedIcon: string
  }
> = {
  default: {
    wrapper: 'justify-end text-primary',
    configure:
      'mr-1 flex h-7 w-7 cursor-pointer items-center justify-center rounded text-amber-400 hover:bg-amber-500/10',
    loading: 'rounded px-1.5 py-0.5 text-sm',
    check:
      'the-check hidden cursor-pointer rounded px-1.5 py-0.5 text-sm hover:bg-primary/15 group-hover:block',
    delay: 'the-delay rounded px-1.5 py-0.5 text-sm',
    selectedIcon: 'the-icon h-4 w-4',
  },
  compact: {
    wrapper: 'ml-1 text-primary',
    configure:
      'mb-1 ml-auto flex h-6 w-6 cursor-pointer items-center justify-center rounded text-amber-400 hover:bg-amber-500/10',
    loading: 'rounded p-0.5 px-1 text-sm',
    check:
      'the-check hidden rounded p-0.5 px-1 text-sm hover:bg-primary/15 group-hover:block',
    delay: 'the-delay rounded p-0.5 px-1 text-sm',
    selectedIcon: 'the-icon mr-1 block h-4 w-4',
  },
}
