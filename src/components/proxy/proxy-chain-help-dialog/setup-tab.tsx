import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import {
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
} from '@/components/tailwind/List'

import { SETUP_ROLES, SETUP_STEPS } from './data'
import { RoleCard } from './shared'

export const ProxyChainHelpSetupTab = () => {
  return (
    <>
      <h6 className="mb-2 text-base font-semibold">如何配置代理链？</h6>

      <Alert severity="info" className="mb-4">
        代理链至少需要 <strong>2 个节点</strong>，也就是入口节点和出口节点。
      </Alert>

      <h6 className="mb-2 text-sm font-semibold">配置步骤</h6>
      <List>
        {SETUP_STEPS.map((item) => (
          <ListItem key={item.step}>
            <ListItemIcon>
              <Chip label={item.step} size="small" color="primary" />
            </ListItemIcon>
            <ListItemText primary={item.title} secondary={item.description} />
          </ListItem>
        ))}
      </List>

      <div className="my-4 border-t border-divider" />

      <h6 className="mb-2 text-sm font-semibold">节点角色说明</h6>
      {SETUP_ROLES.map((item) => (
        <RoleCard key={item.chipLabel} {...item} />
      ))}
    </>
  )
}
