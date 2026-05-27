export const BaseLoading = () => {
  return (
    <div className="relative flex h-full min-h-[18px] items-center">
      <style>{`
        @keyframes loading {
          50% {
            opacity: 0.2;
            transform: scale(0.75);
          }
          100% {
            opacity: 1;
            transform: scale(1);
          }
        }
        .loading-dot {
          animation: loading 0.7s -0.15s infinite linear;
        }
        .loading-dot:nth-child(odd) {
          animation-delay: -0.5s;
        }
      `}</style>
      <div className="loading-dot m-0.5 h-1.5 w-1.5 rounded-full bg-gray-600 dark:bg-gray-400" />
      <div className="loading-dot m-0.5 h-1.5 w-1.5 rounded-full bg-gray-600 dark:bg-gray-400" />
      <div className="loading-dot m-0.5 h-1.5 w-1.5 rounded-full bg-gray-600 dark:bg-gray-400" />
    </div>
  )
}
