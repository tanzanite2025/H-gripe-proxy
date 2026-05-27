# Implementation Plan: Settings Tailwind Migration

## Overview

This implementation plan guides the migration of the Settings module from MUI (Material-UI) to Tailwind CSS. The migration follows a bottom-up, module-by-module approach to ensure stability and functional parity at each step.

**Migration Strategy:**
- Phase 1: Migrate shared foundation components
- Phase 2: Migrate sub-module components by category
- Phase 3: Migrate top-level integration components
- Phase 4: Cleanup and finalization

**Key Principles:**
- Maintain 100% functional parity with existing Settings module
- Preserve visual consistency (or improve where appropriate)
- Use existing 23 Tailwind components from the project library
- Maintain backward compatibility with component APIs
- Ensure all tests pass after each migration step

## Tasks

- [x] 1. Migrate shared foundation components
  - [x] 1.1 Migrate `setting-item.tsx` from MUI to Tailwind
    - Replace MUI `List`, `ListItem`, `ListItemButton`, `ListSubheader` with custom Tailwind implementation
    - Preserve async onClick handling and loading state management
    - Use existing `Box` and `CircularProgress` Tailwind components
    - Maintain identical props interface: `label`, `extra`, `children`, `secondary`, `onClick`
    - _Requirements: 10.1, 10.3, 23.1_
  
  - [ ]* 1.2 Write unit tests for `setting-item.tsx`
    - Test onClick handler invocation
    - Test async operation loading states
    - Test keyboard navigation (Tab, Enter)
    - Test rendering with all prop combinations
    - _Requirements: 21.1, 21.2_
  
  - [x] 1.3 Migrate `password-input.tsx` from MUI to Tailwind
    - Replace MUI `TextField` and `IconButton` with Tailwind equivalents
    - Replace MUI icons (`VisibilityRounded`, `VisibilityOffRounded`) with Lucide icons (`Eye`, `EyeOff`)
    - Preserve show/hide password toggle functionality
    - Maintain identical props interface
    - _Requirements: 10.2, 10.4, 13.1, 13.2_
  
  - [ ]* 1.4 Write unit tests for `password-input.tsx`
    - Test password visibility toggle
    - Test input value changes
    - Test error state display
    - _Requirements: 21.1, 21.2_
  
  - [ ]* 1.5 Create visual regression baseline for shared components
    - Capture screenshots in light and dark modes
    - Capture hover, focus, and disabled states
    - _Requirements: 1.10, 14.6_

- [x] 2. Checkpoint - Verify shared components
  - Ensure all tests pass, ask the user if questions arise.

- [x] 3. Migrate WebUI sub-module components
  - [x] 3.1 Migrate `components/webui/webui-item.tsx`
    - Replace MUI `List`, `ListItem`, `Box` with Tailwind equivalents
    - Preserve WebUI item display logic
    - _Requirements: 2.1, 2.3_
  
  - [x] 3.2 Migrate `components/webui/webui-config.tsx`
    - Replace MUI `Dialog`, `TextField`, `Button` with Tailwind equivalents
    - Preserve form validation logic for WebUI configuration
    - _Requirements: 2.2, 2.3, 2.4_
  
  - [ ]* 3.3 Write integration tests for WebUI configuration flow
    - Test opening and closing WebUI config dialog
    - Test saving WebUI settings
    - Test form validation
    - _Requirements: 21.2, 21.3_

- [x] 4. Migrate Theme sub-module components
  - [x] 4.1 Migrate `components/theme/theme-mode-switch.tsx`
    - Replace MUI `Switch` and `Box` with Tailwind equivalents
    - Preserve theme switching functionality
    - Ensure CSS variables are applied correctly on theme change
    - _Requirements: 3.1, 3.3, 15.1, 15.2_
  
  - [x] 4.2 Migrate `components/theme/theme-config.tsx`
    - Replace MUI `Box`, `Stack`, `Select`, `TextField` with Tailwind equivalents
    - Preserve custom theme color configuration
    - Maintain theme animation effects
    - _Requirements: 3.2, 3.4, 3.5, 15.3_
  
  - [ ]* 4.3 Write integration tests for theme switching
    - Test light to dark mode transition
    - Test custom theme color application
    - Test CSS variable updates
    - Verify color contrast ratios meet WCAG AA standards
    - _Requirements: 15.1, 15.2, 15.4, 15.5, 15.6, 21.2_

- [ ] 5. Migrate Network sub-module components
  - [~] 5.1 Migrate `components/network/tunnels-config.tsx`
    - Replace MUI `Switch`, `TextField`, `Select` with Tailwind equivalents
    - Preserve tunnels configuration logic
    - _Requirements: 4.1, 4.6_
  
  - [~] 5.2 Migrate `components/network/tun-config.tsx`
    - Replace MUI `Switch`, `TextField` with Tailwind equivalents
    - Preserve TUN configuration logic
    - _Requirements: 4.2, 4.6_
  
  - [~] 5.3 Migrate `components/network/network-interface.tsx`
    - Replace MUI `Select`, `TextField` with Tailwind equivalents
    - Preserve network interface selection logic
    - _Requirements: 4.3, 4.6_
  
  - [~] 5.4 Migrate `components/network/external-cors.tsx`
    - Replace MUI `Switch`, `TextField` with Tailwind equivalents
    - Preserve CORS configuration logic
    - _Requirements: 4.4, 4.6_
  
  - [~] 5.5 Migrate `components/network/controller.tsx`
    - Replace MUI `TextField`, `Button` with Tailwind equivalents
    - Preserve controller configuration logic
    - _Requirements: 4.5, 4.6_
  
  - [ ]* 5.6 Write integration tests for network configuration
    - Test network settings validation (IP addresses, ports)
    - Test saving network configuration
    - Test error display for invalid inputs
    - _Requirements: 4.6, 4.7, 18.2, 21.2_

- [~] 6. Checkpoint - Verify network components
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 7. Migrate Proxy sub-module components
  - [~] 7.1 Migrate `components/proxy/system-proxy.tsx`
    - Replace MUI `Switch`, `TextField` with Tailwind equivalents
    - Preserve system proxy configuration logic
    - Maintain form validation for proxy settings
    - _Requirements: 5.1, 5.2, 5.3_
  
  - [ ]* 7.2 Write integration tests for proxy configuration
    - Test proxy settings application
    - Test form validation
    - _Requirements: 21.2_

- [ ] 8. Migrate Misc sub-module components
  - [~] 8.1 Migrate `components/misc/misc-config.tsx`
    - Replace MUI `Box`, `Stack`, `Switch` with Tailwind equivalents
    - Preserve miscellaneous configuration logic
    - _Requirements: 6.1, 6.8_
  
  - [~] 8.2 Migrate `components/misc/update-config.tsx`
    - Replace MUI `Switch`, `Select`, `Button` with Tailwind equivalents
    - Preserve update configuration and progress display
    - _Requirements: 6.2, 6.7, 6.8_
  
  - [x] 8.3 Migrate `components/misc/stack-mode-switch.tsx`
    - Replace MUI `Switch`, `Box` with Tailwind equivalents
    - Preserve stack mode switching logic
    - _Requirements: 6.3, 6.8_
  
  - [~] 8.4 Migrate `components/misc/lite-mode.tsx`
    - Replace MUI `Switch`, `Typography` with Tailwind equivalents
    - Preserve lite mode configuration
    - _Requirements: 6.4, 6.8_
  
  - [~] 8.5 Migrate `components/misc/layout-config.tsx`
    - Replace MUI `Select`, `Switch` with Tailwind equivalents
    - Preserve layout configuration logic
    - _Requirements: 6.5, 6.8_
  
  - [ ] 8.6 Migrate `components/misc/config-editor.tsx`
    - Replace MUI `Dialog`, `TextField`, `Button` with Tailwind equivalents
    - Preserve configuration editor functionality
    - _Requirements: 6.6, 6.8_
  
  - [ ]* 8.7 Write integration tests for misc components
    - Test update progress display
    - Test configuration persistence
    - _Requirements: 6.7, 6.8, 21.2_

- [ ] 9. Migrate Hotkey sub-module components
  - [~] 9.1 Migrate `components/hotkey/hotkey-input.tsx`
    - Replace MUI `TextField`, `Box` with Tailwind equivalents
    - Preserve keyboard input capture logic
    - Maintain custom keyboard event handling
    - _Requirements: 7.1, 7.3, 7.5_
  
  - [~] 9.2 Migrate `components/hotkey/hotkey-config.tsx`
    - Replace MUI `Dialog`, `Box`, `Button` with Tailwind equivalents
    - Preserve hotkey configuration and conflict detection
    - Maintain warning display for conflicting hotkeys
    - _Requirements: 7.2, 7.4, 7.5_
  
  - [ ]* 9.3 Write integration tests for hotkey configuration
    - Test keyboard input capture
    - Test hotkey conflict detection
    - Test warning display
    - _Requirements: 7.3, 7.4, 21.2_

- [~] 10. Checkpoint - Verify hotkey components
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 11. Migrate Backup sub-module components
  - [~] 11.1 Migrate `components/backup/backup-main.tsx`
    - Replace MUI `Box`, `Stack`, `Button` with Tailwind equivalents
    - Preserve backup main interface
    - _Requirements: 11.1, 11.7_
  
  - [~] 11.2 Migrate `components/backup/backup-config.tsx`
    - Replace MUI `Dialog`, `Switch`, `TextField` with Tailwind equivalents
    - Preserve backup configuration logic
    - _Requirements: 11.2, 11.7_
  
  - [~] 11.3 Migrate `components/backup/backup-history.tsx`
    - Replace MUI `List`, `ListItem`, `Button` with custom Tailwind list implementation
    - Preserve backup history display
    - _Requirements: 11.3, 11.7_
  
  - [~] 11.4 Migrate `components/backup/backup-webdav-dialog.tsx`
    - Replace MUI `Dialog`, `TextField`, `Button` with Tailwind equivalents
    - Preserve WebDAV configuration dialog
    - _Requirements: 11.4, 11.7_
  
  - [~] 11.5 Migrate `components/backup/auto-backup-settings.tsx`
    - Replace MUI `Switch`, `Select`, `TextField` with Tailwind equivalents
    - Preserve auto-backup settings logic
    - _Requirements: 11.5, 11.7_
  
  - [ ]* 11.6 Write integration tests for backup functionality
    - Test backup creation
    - Test backup restoration
    - Test backup history display
    - _Requirements: 11.6, 11.7, 11.8, 21.2_

- [ ] 12. Migrate Clash sub-module components
  - [~] 12.1 Migrate `components/clash/clash-core.tsx`
    - Replace MUI `Select`, `Button`, `CircularProgress` with Tailwind equivalents
    - Preserve Clash core switching logic
    - Maintain Clash service restart functionality
    - _Requirements: 9.1, 9.3, 9.5_
  
  - [~] 12.2 Migrate `components/clash/clash-port.tsx`
    - Replace MUI `TextField`, `Box` with Tailwind equivalents
    - Preserve port configuration logic
    - Maintain port number validation (1-65535)
    - _Requirements: 9.2, 9.4, 9.5, 18.1_
  
  - [~] 12.3 Migrate `components/clash/dns-config/index.tsx`
    - Replace MUI `Dialog`, `Tabs`, `Tab`, `Button` with Tailwind equivalents
    - Preserve DNS configuration dialog structure
    - _Requirements: 8.1, 8.7_
  
  - [~] 12.4 Migrate `components/clash/dns-config/components/dns-general-fields.tsx`
    - Replace MUI `Switch`, `TextField`, `Select` with Tailwind equivalents
    - Preserve DNS general configuration fields
    - _Requirements: 8.2, 8.6, 8.8_
  
  - [~] 12.5 Migrate `components/clash/dns-config/components/dns-nameserver-fields.tsx`
    - Replace MUI `TextField`, `Button`, `Chip` with Tailwind equivalents
    - Preserve DNS nameserver configuration
    - Maintain DNS server address validation
    - _Requirements: 8.3, 8.6, 8.8_
  
  - [~] 12.6 Migrate `components/clash/dns-config/components/dns-fallback-fields.tsx`
    - Replace MUI `TextField`, `Button`, `Chip` with Tailwind equivalents
    - Preserve DNS fallback configuration
    - _Requirements: 8.4, 8.6, 8.8_
  
  - [~] 12.7 Migrate `components/clash/dns-config/components/dns-hosts-fields.tsx`
    - Replace MUI `TextField`, `Button`, `Box` with Tailwind equivalents
    - Preserve DNS hosts configuration
    - _Requirements: 8.5, 8.6, 8.8_
  
  - [ ]* 12.8 Write integration tests for Clash components
    - Test Clash core switching
    - Test port validation
    - Test DNS configuration save and load
    - Test DNS server address validation
    - _Requirements: 8.6, 8.7, 9.3, 9.4, 18.1, 18.2, 21.2_

- [~] 13. Checkpoint - Verify all sub-module components
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 14. Migrate top-level Settings components
  - [~] 14.1 Migrate `setting-verge-basic.tsx`
    - Replace MUI `Box`, `Stack`, `Switch`, `TextField` with Tailwind equivalents
    - Verify integration with migrated sub-components
    - Preserve all basic Verge settings functionality
    - _Requirements: 1.1, 1.9, 1.10_
  
  - [~] 14.2 Migrate `setting-verge-advanced.tsx`
    - Replace MUI `Box`, `Stack`, `Switch`, `TextField`, `Select` with Tailwind equivalents
    - Verify integration with migrated sub-components
    - Preserve all advanced Verge settings functionality
    - _Requirements: 1.2, 1.9, 1.10_
  
  - [~] 14.3 Migrate `setting-clash.tsx`
    - Replace MUI `Box`, `Stack`, `Card`, `Divider` with Tailwind equivalents
    - Verify integration with Clash sub-components
    - Preserve Clash settings page layout
    - _Requirements: 1.3, 1.9, 1.10_
  
  - [~] 14.4 Migrate `setting-system.tsx`
    - Replace MUI `Box`, `Stack`, `Switch`, `Select` with Tailwind equivalents
    - Verify integration with system sub-components
    - Preserve system settings functionality
    - _Requirements: 1.4, 1.9, 1.10_
  
  - [~] 14.5 Migrate `dns-stats-card.tsx`
    - Replace MUI `Card`, `Typography`, `CircularProgress` with Tailwind equivalents
    - Preserve DNS statistics display
    - _Requirements: 1.5, 1.9, 1.10_
  
  - [~] 14.6 Migrate `dns-routing-card.tsx`
    - Replace MUI `Card`, `Switch`, `TextField` with Tailwind equivalents
    - Preserve DNS routing configuration
    - _Requirements: 1.6, 1.9, 1.10_
  
  - [~] 14.7 Migrate `dns-leak-protection-card.tsx`
    - Replace MUI `Card`, `Switch`, `Typography` with Tailwind equivalents
    - Preserve DNS leak protection settings
    - _Requirements: 1.7, 1.9, 1.10_
  
  - [~] 14.8 Migrate `tor-config-card.tsx`
    - Replace MUI `Card`, `Switch`, `TextField` with Tailwind equivalents
    - Preserve Tor configuration settings
    - _Requirements: 1.8, 1.9, 1.10_
  
  - [ ]* 14.9 Write integration tests for top-level components
    - Test complete settings save and load flow
    - Test navigation between settings sections
    - Test integration of all sub-components
    - _Requirements: 21.2, 21.3_

- [~] 15. Checkpoint - Verify all top-level components
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 16. Verify responsive layout across all components
  - [~] 16.1 Test desktop layout (≥1024px)
    - Verify multi-column layout renders correctly
    - Test all settings pages at 1920x1080 resolution
    - _Requirements: 16.1, 16.4_
  
  - [~] 16.2 Test tablet layout (768px-1023px)
    - Verify two-column layout renders correctly
    - Test all settings pages at 768x1024 resolution
    - _Requirements: 16.2, 16.4_
  
  - [~] 16.3 Test mobile layout (<768px)
    - Verify single-column layout renders correctly
    - Test all settings pages at 375x667 resolution
    - _Requirements: 16.3, 16.4_
  
  - [ ]* 16.4 Create responsive layout snapshot tests
    - Capture snapshots at all breakpoints
    - Verify Tailwind responsive classes are applied correctly
    - _Requirements: 16.5, 21.4_

- [ ] 17. Verify animation and transition effects
  - [~] 17.1 Implement dialog animations
    - Use Framer Motion or Tailwind transition classes for dialog open/close
    - Ensure fade-in and scale animations (duration: 200-300ms)
    - _Requirements: 17.1, 17.2, 17.5_
  
  - [~] 17.2 Implement collapse/expand animations
    - Use Framer Motion for smooth height transitions
    - Ensure animation duration is 200-300ms
    - _Requirements: 17.1, 17.3, 17.5_
  
  - [~] 17.3 Implement button hover effects
    - Use Tailwind transition classes for color and shadow transitions
    - Ensure smooth hover effects (duration: 150ms)
    - _Requirements: 17.1, 17.4, 17.5_
  
  - [ ]* 17.4 Write animation timing tests
    - Verify animation durations are within acceptable range
    - Test transition properties are applied correctly
    - _Requirements: 17.5, 21.2_

- [ ] 18. Verify accessibility compliance
  - [~] 18.1 Add ARIA labels to all interactive elements
    - Ensure all buttons have aria-label or aria-labelledby
    - Ensure all form fields have associated labels
    - Ensure dialogs have role="dialog" and aria-modal="true"
    - _Requirements: 19.1, 19.3, 19.4, 19.5_
  
  - [~] 18.2 Verify keyboard navigation
    - Test Tab navigation through all interactive elements
    - Test Enter key activation for buttons
    - Test Escape key for closing dialogs
    - _Requirements: 19.2_
  
  - [ ]* 18.3 Run automated accessibility tests
    - Use jest-axe to check for accessibility violations
    - Ensure zero violations for all components
    - _Requirements: 19.6, 21.2_

- [ ] 19. Cleanup and finalization
  - [~] 19.1 Remove all MUI imports from Settings module
    - Search for and remove all `@mui/material` imports
    - Search for and remove all `@mui/icons-material` imports
    - Search for and remove all `@emotion/react` and `@emotion/styled` imports
    - _Requirements: 20.1, 20.2, 25.5_
  
  - [~] 19.2 Remove unused MUI-related code
    - Remove any MUI theme provider wrappers
    - Remove MUI-specific utility functions
    - Clean up any Emotion sx prop remnants
    - _Requirements: 20.1, 20.2, 25.5_
  
  - [~] 19.3 Run code quality checks
    - Run ESLint and fix all issues
    - Run TypeScript type checking and fix all errors
    - Run Biome formatter and apply formatting
    - _Requirements: 25.1, 25.2, 25.3, 25.4_
  
  - [~] 19.4 Verify bundle size reduction
    - Build production bundle
    - Compare Settings module bundle size before and after migration
    - Verify at least 50% reduction in bundle size
    - _Requirements: 20.4_
  
  - [~] 19.5 Verify runtime style injection elimination
    - Run application and inspect DOM
    - Verify no Emotion-generated `<style>` tags are present
    - Verify all styles are compiled at build time
    - _Requirements: 20.3, 20.5_
  
  - [ ]* 19.6 Run full regression test suite
    - Run all unit tests
    - Run all integration tests
    - Run all visual regression tests
    - Run all accessibility tests
    - _Requirements: 21.1, 21.2, 21.3, 21.4_

- [ ] 20. Update documentation
  - [~] 20.1 Update `TAILWIND_MIGRATION_PROGRESS.md`
    - Mark Settings module as complete
    - Update migration statistics
    - _Requirements: 22.1_
  
  - [~] 20.2 Create `SETTINGS_MIGRATION_COMPLETE.md`
    - Document migration approach and decisions
    - List all migrated components
    - Document any breaking changes or caveats
    - Include before/after bundle size comparison
    - _Requirements: 22.2_
  
  - [~] 20.3 Update `TAILWIND_COMPONENT_LIBRARY.md`
    - Add Settings component usage examples
    - Document any new patterns discovered during migration
    - _Requirements: 22.3_
  
  - [~] 20.4 Update `README.md`
    - Remove MUI-related setup instructions
    - Update dependencies section
    - Update styling architecture documentation
    - _Requirements: 22.4_
  
  - [~] 20.5 Add code comments for major changes
    - Document significant architectural changes in code comments
    - Explain any complex style conversions
    - _Requirements: 22.5_

- [~] 21. Final checkpoint - Complete migration verification
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation at key milestones
- The migration follows a bottom-up approach: shared components → sub-modules → top-level components
- All component APIs remain unchanged to maintain backward compatibility
- Visual regression tests ensure no unintended visual changes
- Integration tests verify functional equivalence with MUI version
- Accessibility tests ensure WCAG AA compliance is maintained

## Task Dependency Graph

```json
{
  "waves": [
    {
      "id": 0,
      "tasks": ["1.1", "1.3"]
    },
    {
      "id": 1,
      "tasks": ["1.2", "1.4", "1.5"]
    },
    {
      "id": 2,
      "tasks": ["3.1", "3.2", "4.1", "4.2"]
    },
    {
      "id": 3,
      "tasks": ["3.3", "4.3", "5.1", "5.2", "5.3", "5.4", "5.5"]
    },
    {
      "id": 4,
      "tasks": ["5.6", "7.1", "8.1", "8.2", "8.3", "8.4", "8.5", "8.6"]
    },
    {
      "id": 5,
      "tasks": ["7.2", "8.7", "9.1", "9.2"]
    },
    {
      "id": 6,
      "tasks": ["9.3", "11.1", "11.2", "11.3", "11.4", "11.5"]
    },
    {
      "id": 7,
      "tasks": ["11.6", "12.1", "12.2", "12.3", "12.4", "12.5", "12.6", "12.7"]
    },
    {
      "id": 8,
      "tasks": ["12.8", "14.1", "14.2", "14.3", "14.4", "14.5", "14.6", "14.7", "14.8"]
    },
    {
      "id": 9,
      "tasks": ["14.9", "16.1", "16.2", "16.3"]
    },
    {
      "id": 10,
      "tasks": ["16.4", "17.1", "17.2", "17.3"]
    },
    {
      "id": 11,
      "tasks": ["17.4", "18.1", "18.2"]
    },
    {
      "id": 12,
      "tasks": ["18.3", "19.1", "19.2", "19.3"]
    },
    {
      "id": 13,
      "tasks": ["19.4", "19.5"]
    },
    {
      "id": 14,
      "tasks": ["19.6", "20.1", "20.2", "20.3", "20.4", "20.5"]
    }
  ]
}
```
