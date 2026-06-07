import { getRecommendedAdvancedConfig } from '@/services/coordinator'
import { showNotice } from '@/services/notice-service'

import type {
  AppEgressRule,
  EgressIdentityConfig,
  EgressIdentityProfile,
  ShortcutEgressRule,
} from '@/services/coordinator'
import {
  buildProfileId,
  starterAppRule,
  starterProfile,
  starterShortcutRule,
} from './defaults'

interface UseEgressConfigEditorParams {
  config: EgressIdentityConfig
  onChange: (config: EgressIdentityConfig) => void
}

export function useEgressConfigEditor({
  config,
  onChange,
}: UseEgressConfigEditorParams) {
  const updateConfig = (nextConfig: EgressIdentityConfig) => {
    onChange(nextConfig)
  }

  const ensureInitialized = async () => {
    if (
      config.profiles.length > 0 ||
      config.app_rules.length > 0 ||
      config.shortcut_rules.length > 0
    ) {
      const initializedProfiles =
        config.profiles.length > 0 ? config.profiles : [starterProfile]
      const defaultProfile = config.default_profile || initializedProfiles[0]?.id || null

      updateConfig({
        ...config,
        enabled: true,
        default_profile: defaultProfile,
        profiles: initializedProfiles,
        app_rules: config.app_rules.length > 0 ? config.app_rules : [starterAppRule],
        shortcut_rules:
          config.shortcut_rules.length > 0
            ? config.shortcut_rules
            : [starterShortcutRule],
      })
      return
    }

    try {
      const recommended = await getRecommendedAdvancedConfig()
      updateConfig({
        ...recommended.egress_identity,
        enabled: true,
      })
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '加载推荐出口身份配置失败',
      )
    }
  }

  const applyProfileReferenceUpdate = (
    oldProfileId: string,
    nextProfileId: string,
    nextProfiles: EgressIdentityProfile[],
  ) => {
    updateConfig({
      ...config,
      default_profile:
        config.default_profile === oldProfileId
          ? nextProfileId || null
          : config.default_profile,
      profiles: nextProfiles,
      app_rules: config.app_rules.map((rule) =>
        rule.profile_id === oldProfileId
          ? { ...rule, profile_id: nextProfileId }
          : rule,
      ),
      shortcut_rules: config.shortcut_rules.map((rule) =>
        rule.profile_id === oldProfileId
          ? { ...rule, profile_id: nextProfileId }
          : rule,
      ),
    })
  }

  const addProfile = () => {
    const nextId = buildProfileId(config.profiles.map((profile) => profile.id))
    const nextProfile: EgressIdentityProfile = {
      ...starterProfile,
      id: nextId,
      name: `画像 ${config.profiles.length + 1}`,
    }

    updateConfig({
      ...config,
      profiles: [...config.profiles, nextProfile],
      default_profile: config.default_profile || nextId,
    })
  }

  const updateProfile = (
    profileId: string,
    updater: (profile: EgressIdentityProfile) => EgressIdentityProfile,
  ) => {
    updateConfig({
      ...config,
      profiles: config.profiles.map((profile) =>
        profile.id === profileId ? updater(profile) : profile,
      ),
    })
  }

  const renameProfileId = (profileId: string, nextProfileId: string) => {
    const nextProfiles = config.profiles.map((profile) =>
      profile.id === profileId ? { ...profile, id: nextProfileId } : profile,
    )
    applyProfileReferenceUpdate(profileId, nextProfileId, nextProfiles)
  }

  const removeProfile = (profileId: string) => {
    const remainingProfiles = config.profiles.filter(
      (profile) => profile.id !== profileId,
    )
    const fallbackProfileId = remainingProfiles[0]?.id || null

    updateConfig({
      ...config,
      default_profile:
        config.default_profile === profileId
          ? fallbackProfileId
          : config.default_profile,
      profiles: remainingProfiles,
      app_rules: fallbackProfileId
        ? config.app_rules.map((rule) =>
            rule.profile_id === profileId
              ? { ...rule, profile_id: fallbackProfileId }
              : rule,
          )
        : config.app_rules.filter((rule) => rule.profile_id !== profileId),
      shortcut_rules: fallbackProfileId
        ? config.shortcut_rules.map((rule) =>
            rule.profile_id === profileId
              ? { ...rule, profile_id: fallbackProfileId }
              : rule,
          )
        : config.shortcut_rules.filter((rule) => rule.profile_id !== profileId),
    })
  }

  const addAppRule = () => {
    const profileId = config.default_profile || config.profiles[0]?.id

    if (!profileId) {
      showNotice('error', '请先添加至少一个出口画像')
      return
    }

    updateConfig({
      ...config,
      app_rules: [
        ...config.app_rules,
        {
          ...starterAppRule,
          profile_id: profileId,
        },
      ],
    })
  }

  const updateAppRule = (index: number, nextRule: AppEgressRule) => {
    const nextRules = [...config.app_rules]
    nextRules[index] = nextRule
    updateConfig({ ...config, app_rules: nextRules })
  }

  const removeAppRule = (index: number) => {
    updateConfig({
      ...config,
      app_rules: config.app_rules.filter(
        (_, currentIndex) => currentIndex !== index,
      ),
    })
  }

  const addShortcutRule = () => {
    const profileId = config.default_profile || config.profiles[0]?.id

    if (!profileId) {
      showNotice('error', '请先添加至少一个出口画像')
      return
    }

    updateConfig({
      ...config,
      shortcut_rules: [
        ...config.shortcut_rules,
        {
          ...starterShortcutRule,
          profile_id: profileId,
          shortcut_id: `shortcut-${config.shortcut_rules.length + 1}`,
        },
      ],
    })
  }

  const updateShortcutRule = (
    index: number,
    nextRule: ShortcutEgressRule,
  ) => {
    const nextRules = [...config.shortcut_rules]
    nextRules[index] = nextRule
    updateConfig({ ...config, shortcut_rules: nextRules })
  }

  const removeShortcutRule = (index: number) => {
    updateConfig({
      ...config,
      shortcut_rules: config.shortcut_rules.filter(
        (_, currentIndex) => currentIndex !== index,
      ),
    })
  }

  const handleToggleEnabled = (enabled: boolean) => {
    updateConfig({ ...config, enabled })
  }

  const handleClearRules = () => {
    updateConfig({
      ...config,
      app_rules: [],
      shortcut_rules: [],
    })
  }

  const handleDefaultProfileChange = (profileId: string) => {
    updateConfig({
      ...config,
      default_profile: profileId || null,
    })
  }

  return {
    ensureInitialized,
    addProfile,
    updateProfile,
    renameProfileId,
    removeProfile,
    addAppRule,
    updateAppRule,
    removeAppRule,
    addShortcutRule,
    updateShortcutRule,
    removeShortcutRule,
    handleToggleEnabled,
    handleClearRules,
    handleDefaultProfileChange,
  }
}
