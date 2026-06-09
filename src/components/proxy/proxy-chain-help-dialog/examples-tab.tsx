import { EXAMPLE_CARDS } from './data'
import { ExampleCard } from './shared'

export const ProxyChainHelpExamplesTab = () => {
  return (
    <>
      <h6 className="mb-2 text-base font-semibold">配置示例</h6>
      {EXAMPLE_CARDS.map((item) => (
        <ExampleCard key={item.title} {...item} />
      ))}
    </>
  )
}
