interface EmptyStateProps {
  message: string
}

export function EmptyState({ message }: EmptyStateProps) {
  return <div className="py-8 text-center text-sm text-gray-500">{message}</div>
}
