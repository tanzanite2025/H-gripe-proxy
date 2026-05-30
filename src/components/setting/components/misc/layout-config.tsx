import { forwardRef, useImperativeHandle, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef, Switch, TooltipIcon } from '@/components/base'
import { DEFAULT_HOVER_DELAY } from '@/components/proxy/proxy-group-navigator'
import {
  Box,
  InputAdornment,
  List,
  ListItem,
  ListItemText,
  SelectMenuItem,
  Select,
  TextField,
} from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import { useWindowDecorations } from '@/hooks/ui'
import { showNotice } from '@/services/notice-service'
import getSystem from '@/utils/misc'

import { GuardState } from '../proxy/guard-state'

const OS = getSystem()

const clampHoverDelay = (value: number) => {
  if (!Number.isFinite(value)) {
    return DEFAULT_HOVER_DELAY
  }
  return Math.min(5000, Math.max(0, Math.round(value)))
}

export const LayoutViewer = forwardRef<DialogRef>((_, ref) => {
  const { t } = useTranslation()
  const { verge, patchVerge, mutateVerge } = useVerge()

  const [open, setOpen] = useState(false)

  const { decorated, toggleDecorations } = useWindowDecorations()

  useImperativeHandle(ref, () => ({
    open: () => setOpen(true),
    close: () => setOpen(false),
  }))

  const onSwitchFormat = (_e: any, value: boolean) => value
  const onError = (err: any) => {
    showNotice.error(err)
  }
  const onChangeData = (patch: Partial<IVergeConfig>) => {
    mutateVerge({ ...verge, ...patch }, false)
  }

  return (
    <BaseDialog
      open={open}
      title={t('settings.components.verge.layout.title')}
      panelStyle={{ width: 600, maxWidth: 600 }}
      disableOk
      cancelBtn={t('shared.actions.close')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
    >
      <List>
        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t(
              'settings.components.verge.layout.fields.preferSystemTitlebar',
            )}
          />
          <GuardState
            value={decorated}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={async () => {
              await toggleDecorations()
            }}
          >
            <Switch />
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t('settings.components.verge.layout.fields.trafficGraph')}
          />
          <GuardState
            value={verge?.traffic_graph ?? true}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) => onChangeData({ traffic_graph: e })}
            onGuard={(e) => patchVerge({ traffic_graph: e })}
          >
            <Switch />
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t('settings.components.verge.layout.fields.memoryUsage')}
          />
          <GuardState
            value={verge?.enable_memory_usage ?? true}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) => onChangeData({ enable_memory_usage: e })}
            onGuard={(e) => patchVerge({ enable_memory_usage: e })}
          >
            <Switch />
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t(
              'settings.components.verge.layout.fields.proxyGroupIcon',
            )}
          />
          <GuardState
            value={verge?.enable_group_icon ?? true}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) => onChangeData({ enable_group_icon: e })}
            onGuard={(e) => patchVerge({ enable_group_icon: e })}
          >
            <Switch />
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t(
              'settings.components.verge.layout.fields.pauseRenderTrafficStatsOnBlur',
            )}
          />
          <GuardState
            value={verge?.pause_render_traffic_stats_on_blur ?? true}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) =>
              onChangeData({ pause_render_traffic_stats_on_blur: e })
            }
            onGuard={(e) =>
              patchVerge({ pause_render_traffic_stats_on_blur: e })
            }
          >
            <Switch />
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t('settings.components.verge.layout.fields.toastPosition')}
          />
          <GuardState
            value={verge?.notice_position ?? 'top-right'}
            onCatch={onError}
            onFormat={(e: any) => e.target.value}
            onChange={(value) => onChangeData({ notice_position: value })}
            onGuard={(value) => patchVerge({ notice_position: value })}
          >
            <Select size="small" className="w-[180px]">
              <SelectMenuItem value="top-right">
                {t(
                  'settings.components.verge.layout.options.toastPosition.topRight',
                )}
              </SelectMenuItem>
              <SelectMenuItem value="top-left">
                {t(
                  'settings.components.verge.layout.options.toastPosition.topLeft',
                )}
              </SelectMenuItem>
              <SelectMenuItem value="bottom-right">
                {t(
                  'settings.components.verge.layout.options.toastPosition.bottomRight',
                )}
              </SelectMenuItem>
              <SelectMenuItem value="bottom-left">
                {t(
                  'settings.components.verge.layout.options.toastPosition.bottomLeft',
                )}
              </SelectMenuItem>
            </Select>
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={
              <Box className="flex items-center gap-2">
                <span>
                  {t('settings.components.verge.layout.fields.hoverNavigator')}
                </span>
                <TooltipIcon
                  title={t(
                    'settings.components.verge.layout.tooltips.hoverNavigator',
                  )}
                  className="opacity-70"
                />
              </Box>
            }
          />
          <GuardState
            value={verge?.enable_hover_jump_navigator ?? true}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) => onChangeData({ enable_hover_jump_navigator: e })}
            onGuard={(e) => patchVerge({ enable_hover_jump_navigator: e })}
          >
            <Switch />
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={
              <Box className="flex items-center gap-2">
                <span>
                  {t(
                    'settings.components.verge.layout.fields.hoverNavigatorDelay',
                  )}
                </span>
                <TooltipIcon
                  title={t(
                    'settings.components.verge.layout.tooltips.hoverNavigatorDelay',
                  )}
                  className="opacity-70"
                />
              </Box>
            }
          />
          <GuardState
            value={verge?.hover_jump_navigator_delay ?? DEFAULT_HOVER_DELAY}
            waitTime={400}
            onCatch={onError}
            onFormat={(e: any) => clampHoverDelay(Number(e.target.value))}
            onChange={(value) =>
              onChangeData({
                hover_jump_navigator_delay: clampHoverDelay(value),
              })
            }
            onGuard={(value) =>
              patchVerge({ hover_jump_navigator_delay: clampHoverDelay(value) })
            }
          >
            <TextField
              type="number"
              size="small"
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck={false}
              className="w-[120px]"
              disabled={!(verge?.enable_hover_jump_navigator ?? true)}
              slotProps={{
                input: {
                  endAdornment: (
                    <InputAdornment position="end">
                      {t('shared.units.milliseconds')}
                    </InputAdornment>
                  ),
                },
                htmlInput: {
                  min: 0,
                  max: 5000,
                  step: 20,
                },
              }}
            />
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t('settings.components.verge.layout.fields.navIcon')}
          />
          <GuardState
            value={verge?.menu_icon ?? 'monochrome'}
            onCatch={onError}
            onFormat={(e: any) => e.target.value}
            onChange={(value) => onChangeData({ menu_icon: value })}
            onGuard={(value) => patchVerge({ menu_icon: value })}
          >
            <Select size="small" className="w-[140px]">
              <SelectMenuItem value="monochrome">
                {t('settings.components.verge.layout.options.icon.monochrome')}
              </SelectMenuItem>
              <SelectMenuItem value="colorful">
                {t('settings.components.verge.layout.options.icon.colorful')}
              </SelectMenuItem>
              <SelectMenuItem value="disable">
                {t('settings.components.verge.layout.options.icon.disable')}
              </SelectMenuItem>
            </Select>
          </GuardState>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t(
              'settings.components.verge.layout.fields.collapseNavBar',
            )}
          />
          <GuardState
            value={verge?.collapse_navbar ?? false}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) => onChangeData({ collapse_navbar: e })}
            onGuard={(e) => patchVerge({ collapse_navbar: e })}
          >
            <Switch />
          </GuardState>
        </ListItem>

        {OS === 'macos' && (
          <ListItem className="py-[5px] px-[2px]">
            <ListItemText
              primary={t('settings.components.verge.layout.fields.trayIcon')}
            />
            <GuardState
              value={verge?.tray_icon ?? 'monochrome'}
              onCatch={onError}
              onFormat={(e: any) => e.target.value}
              onChange={(e) => onChangeData({ tray_icon: e })}
              onGuard={(e) => patchVerge({ tray_icon: e })}
            >
              <Select size="small" className="w-[140px]">
                <SelectMenuItem value="monochrome">
                  {t(
                    'settings.components.verge.layout.options.icon.monochrome',
                  )}
                </SelectMenuItem>
                <SelectMenuItem value="colorful">
                  {t('settings.components.verge.layout.options.icon.colorful')}
                </SelectMenuItem>
              </Select>
            </GuardState>
          </ListItem>
        )}
        {OS === 'macos' && (
          <ListItem className="py-[5px] px-[2px]">
            <ListItemText
              primary={t(
                'settings.components.verge.layout.fields.enableTraySpeed',
              )}
            />
            <GuardState
              value={verge?.enable_tray_speed ?? false}
              valueProps="checked"
              onCatch={onError}
              onFormat={onSwitchFormat}
              onChange={(e) => onChangeData({ enable_tray_speed: e })}
              onGuard={(e) => patchVerge({ enable_tray_speed: e })}
            >
              <Switch />
            </GuardState>
          </ListItem>
        )}
        {/* {OS === "macos" && (
          <ListItem className="py-[5px] px-[2px]">
            <ListItemText primary={t("settings.components.verge.layout.fields.enableTrayIcon")} />
            <GuardState
              value={
                verge?.enable_tray_icon === false &&
                verge?.enable_tray_speed === false
                  ? true
                  : (verge?.enable_tray_icon ?? true)
              }
              valueProps="checked"
              onCatch={onError}
              onFormat={onSwitchFormat}
              onChange={(e) => onChangeData({ enable_tray_icon: e })}
              onGuard={(e) => patchVerge({ enable_tray_icon: e })}
            >
              <Switch />
            </GuardState>
          </ListItem>
        )} */}
        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t(
              'settings.components.verge.layout.fields.proxyGroupsDisplayMode',
            )}
          />
          <GuardState
            value={verge?.tray_proxy_groups_display_mode ?? 'default'}
            onCatch={onError}
            onFormat={(e: any) => e.target.value}
            onChange={(value) =>
              onChangeData({ tray_proxy_groups_display_mode: value })
            }
            onGuard={(value) =>
              patchVerge({ tray_proxy_groups_display_mode: value })
            }
          >
            <Select size="small" className="w-[140px]">
              <SelectMenuItem value="default">
                {t(
                  'settings.components.verge.layout.options.proxyGroupsDisplayMode.default',
                )}
              </SelectMenuItem>
              <SelectMenuItem value="inline">
                {t(
                  'settings.components.verge.layout.options.proxyGroupsDisplayMode.inline',
                )}
              </SelectMenuItem>
              <SelectMenuItem value="disable">
                {t(
                  'settings.components.verge.layout.options.proxyGroupsDisplayMode.disable',
                )}
              </SelectMenuItem>
            </Select>
          </GuardState>
        </ListItem>
        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t(
              'settings.components.verge.layout.fields.showOutboundModesInline',
            )}
          />
          <GuardState
            value={verge?.tray_inline_outbound_modes ?? false}
            valueProps="checked"
            onCatch={onError}
            onFormat={onSwitchFormat}
            onChange={(e) => onChangeData({ tray_inline_outbound_modes: e })}
            onGuard={(e) => patchVerge({ tray_inline_outbound_modes: e })}
          >
            <Switch />
          </GuardState>
        </ListItem>

      </List>
    </BaseDialog>
  )
})
