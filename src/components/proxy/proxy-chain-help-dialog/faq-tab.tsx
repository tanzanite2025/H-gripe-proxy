import { FAQ_CARDS } from './data'
import { FaqCard } from './shared'

export const ProxyChainHelpFaqTab = () => {
  return (
    <>
      <h6 className="mb-2 text-base font-semibold">常见问题</h6>
      {FAQ_CARDS.map((item) => (
        <FaqCard key={item.question} {...item} />
      ))}
    </>
  )
}
