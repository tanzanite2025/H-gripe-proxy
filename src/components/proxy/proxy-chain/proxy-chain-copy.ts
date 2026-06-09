export interface ProxyChainCopy {
  entryLabel: string
  exitLabel: string
  timeoutLabel: string
  header: string
  warning: string
  instruction: string
  minimumNodesMessage: string
  disconnectFailedMessage: string
  connectFailedMessage: string
  clearChainLabel: string
  connectingLabel: string
  disconnectLabel: string
  connectLabel: string
  emptyLabel: string
  helpLabel: string
  residentialPoolLabel: string
}

const getTranslatedLabel = (
  t: (key: string) => string,
  key: string,
  fallback: string,
) => {
  const translated = t(key)
  return !translated || translated === key ? fallback : translated
}

export const buildProxyChainCopy = (
  t: (key: string) => string,
  chainLength: number,
): ProxyChainCopy => ({
  entryLabel: getTranslatedLabel(t, 'proxies.page.chain.entryNode', '入口节点'),
  exitLabel: getTranslatedLabel(t, 'proxies.page.chain.exitNode', '出口节点'),
  timeoutLabel: getTranslatedLabel(t, 'shared.labels.timeout', '超时'),
  header: getTranslatedLabel(t, 'proxies.page.chain.header', '代理链'),
  warning: getTranslatedLabel(
    t,
    'proxies.page.chain.warning',
    '代理链会增加延迟，并且整条链的稳定性取决于最慢或最不稳定的节点。',
  ),
  instruction:
    chainLength === 1
      ? getTranslatedLabel(
          t,
          'proxies.page.chain.minimumNodesHint',
          '代理链至少需要 2 个节点，请再添加一个节点。',
        )
      : getTranslatedLabel(
          t,
          'proxies.page.chain.instruction',
          '按顺序点击节点，把它们添加到代理链里。',
        ),
  minimumNodesMessage: getTranslatedLabel(
    t,
    'proxies.page.chain.minimumNodes',
    '代理链至少需要 2 个节点',
  ),
  disconnectFailedMessage: getTranslatedLabel(
    t,
    'proxies.page.chain.disconnectFailed',
    '断开代理链失败',
  ),
  connectFailedMessage: getTranslatedLabel(
    t,
    'proxies.page.chain.connectFailed',
    '连接代理链失败',
  ),
  clearChainLabel: getTranslatedLabel(
    t,
    'proxies.page.actions.clearChainConfig',
    '清除链式配置',
  ),
  connectingLabel: getTranslatedLabel(
    t,
    'proxies.page.actions.connecting',
    '连接中...',
  ),
  disconnectLabel: getTranslatedLabel(
    t,
    'proxies.page.actions.disconnect',
    '断开',
  ),
  connectLabel: getTranslatedLabel(
    t,
    'proxies.page.actions.connect',
    '连接',
  ),
  emptyLabel: getTranslatedLabel(
    t,
    'proxies.page.chain.empty',
    '暂时还没有添加节点',
  ),
  helpLabel: '使用帮助',
  residentialPoolLabel: '住宅代理池配置',
})
