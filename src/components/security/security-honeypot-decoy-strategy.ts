import {
  type HoneypotDecoy,
  type NewHoneypotDecoyInput,
  addHoneypotDecoy,
} from './security-honeypot-decoys'

export interface HoneypotDecoyStrategyProfile {
  prefix: string
  directories: string[]
  filenames: string[]
  enabled: boolean
}

export function createHoneypotDecoyStrategyProfile(
  profile?: Partial<HoneypotDecoyStrategyProfile>,
): HoneypotDecoyStrategyProfile {
  return {
    prefix: profile?.prefix ?? 'dynamic',
    directories: profile?.directories ?? ['profiles', 'providers', 'rules'],
    filenames: profile?.filenames ?? ['config_decoy.yaml'],
    enabled: profile?.enabled ?? true,
  }
}

export function generateHoneypotDecoysFromStrategy(
  profile: HoneypotDecoyStrategyProfile,
): NewHoneypotDecoyInput[] {
  return profile.directories.flatMap((directory) =>
    profile.filenames.map((filename) => ({
      name: `${profile.prefix} ${directory} ${filename}`,
      path: `${directory}/${filename}`,
      enabled: profile.enabled,
    })),
  )
}

export function mergeHoneypotDecoyStrategy(
  decoys: HoneypotDecoy[],
  profile: HoneypotDecoyStrategyProfile,
): HoneypotDecoy[] {
  return generateHoneypotDecoysFromStrategy(profile).reduce(
    (nextDecoys, input) => {
      if (nextDecoys.some((decoy) => decoy.path === input.path)) {
        return nextDecoys
      }

      return addHoneypotDecoy(nextDecoys, input)
    },
    decoys,
  )
}
