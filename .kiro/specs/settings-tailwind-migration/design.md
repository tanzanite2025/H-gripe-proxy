# Design Document: Settings Tailwind Migration

## Overview

This document defines the technical design for migrating the Settings module from MUI (Material-UI) to Tailwind CSS. This migration represents the final phase of the application-wide Tailwind CSS adoption, completing the transition from a three-layer style architecture (MUI + Emotion + SCSS) to a unified single-layer Tailwind CSS architecture.

### Context

The main application has successfully migrated 10 pages and the Layout component to Tailwind CSS, achieving:
- 80% bundle size reduction (2.5MB → 500KB)
- Elimination of runtime CSS-in-JS overhead
- Unified style architecture

The Settings module is the last major component using MUI, comprising:
- 41 total files
- ~35 files using MUI components
- 8 top-level components
- 7 sub-module categories (webui, theme, network, proxy, misc, hotkey, backup, clash)

### Goals

1. **Complete Migration**: Convert all 35 MUI-dependent files to use Tailwind components
2. **Functional Parity**: Maintain 100% feature compatibility with existing Settings module
3. **Visual Consistency**: Preserve or improve the current visual design
4. **Performance Optimization**: Eliminate MUI/Emotion dependencies and reduce bundle size by ≥50%
5. **Code Quality**: Maintain or improve code maintainability and readability

### Non-Goals

1. Adding new features to the Settings module
2. Refactoring business logic or state management
3. Changing the Settings module's public API
4. Modifying the Settings module's file structure


## Architecture

### Current Architecture (Before Migration)

```
Settings Module
├── Style Layer 1: MUI Components (@mui/material)
│   ├── Button, TextField, Select, Switch, Dialog
│   ├── Box, Stack, Grid (layout)
│   └── Typography, Divider, CircularProgress
├── Style Layer 2: Emotion CSS-in-JS (@emotion/react)
│   ├── Runtime style injection
│   ├── sx prop processing
│   └── Theme variable resolution
└── Style Layer 3: SCSS (settings.scss)
    ├── Custom component styles
    └── Layout overrides
```

**Issues with Current Architecture:**
- Three competing style systems
- Runtime CSS-in-JS overhead (~150ms initial render penalty)
- Large bundle size (~1.2MB for MUI + Emotion)
- Style injection causes FOUC (Flash of Unstyled Content)
- Difficult to maintain consistent styling

### Target Architecture (After Migration)

```
Settings Module
└── Single Style Layer: Tailwind CSS
    ├── Compile-time CSS generation
    ├── Utility-first classes
    ├── CSS variables for theming
    └── Zero runtime overhead
```


**Benefits of Target Architecture:**
- Single source of truth for styling
- Zero runtime overhead (all CSS compiled at build time)
- Smaller bundle size (estimated 50-70% reduction)
- Consistent styling across entire application
- Better developer experience with IntelliSense support

### Migration Strategy

The migration follows a **bottom-up, module-by-module** approach:

1. **Phase 1: Shared Components** (Foundation)
   - Migrate `setting-item.tsx` and `password-input.tsx`
   - These are used by all other components

2. **Phase 2: Sub-modules** (By category)
   - WebUI components (2 files)
   - Theme components (2 files)
   - Network components (5 files)
   - Proxy components (1 file)
   - Misc components (6 files)
   - Hotkey components (2 files)
   - Backup components (5 files)
   - Clash components (7 files including DNS config)

3. **Phase 3: Top-level Components** (Integration)
   - Migrate 8 top-level setting pages
   - Verify integration with sub-modules

4. **Phase 4: Cleanup** (Finalization)
   - Remove MUI imports
   - Update documentation
   - Run comprehensive tests


## Components and Interfaces

### Existing Tailwind Component Library

The project has 23 pre-built Tailwind components available for use:

| Component | Purpose | MUI Equivalent |
|-----------|---------|----------------|
| `Button` | Action buttons | `Button` |
| `TextField` | Text input | `TextField`, `Input` |
| `Select` | Dropdown selection | `Select`, `MenuItem` |
| `Switch` | Toggle switch | `Switch` |
| `Dialog` | Modal dialogs | `Dialog` |
| `Tooltip` | Hover tooltips | `Tooltip` |
| `IconButton` | Icon-only buttons | `IconButton` |
| `Box` | Layout container | `Box` |
| `Stack` | Flex layout | `Stack` |
| `Divider` | Visual separator | `Divider` |
| `Typography` | Text display | `Typography` |
| `CircularProgress` | Loading spinner | `CircularProgress` |
| `Alert` | Alert messages | `Alert` |
| `Chip` | Tag/label display | `Chip` |
| `ButtonGroup` | Grouped buttons | `ButtonGroup` |
| `Tabs`, `Tab` | Tab navigation | `Tabs`, `Tab` |
| `Card` | Card container | `Card` |
| `Skeleton` | Loading placeholder | `Skeleton` |
| `Menu`, `MenuItem` | Context menus | `Menu`, `MenuItem` |
| `Grid` | Grid layout | `Grid` |
| `Fab` | Floating action button | `Fab` |
| `Zoom` | Zoom animation | `Zoom` |


### Component Migration Mapping

#### Top-Level Components (8 files)

| File | MUI Components Used | Tailwind Replacements | Complexity |
|------|---------------------|----------------------|------------|
| `setting-verge-basic.tsx` | Box, Stack, Switch, TextField | Box, Stack, Switch, TextField | Medium |
| `setting-verge-advanced.tsx` | Box, Stack, Switch, TextField, Select | Box, Stack, Switch, TextField, Select | Medium |
| `setting-clash.tsx` | Box, Stack, Card, Divider | Box, Stack, Card, Divider | Low |
| `setting-system.tsx` | Box, Stack, Switch, Select | Box, Stack, Switch, Select | Low |
| `dns-stats-card.tsx` | Card, Typography, CircularProgress | Card, Typography, CircularProgress | Low |
| `dns-routing-card.tsx` | Card, Switch, TextField | Card, Switch, TextField | Medium |
| `dns-leak-protection-card.tsx` | Card, Switch, Typography | Card, Switch, Typography | Low |
| `tor-config-card.tsx` | Card, Switch, TextField | Card, Switch, TextField | Medium |

#### Shared Components (2 files)

| File | MUI Components Used | Tailwind Replacements | Complexity |
|------|---------------------|----------------------|------------|
| `setting-item.tsx` | List, ListItem, ListItemButton, ListSubheader, Box, CircularProgress | Custom implementation with Box, CircularProgress | High |
| `password-input.tsx` | TextField, IconButton | TextField, IconButton | Low |


#### Sub-Module Components (25 files)

**WebUI (2 files)**
- `webui-item.tsx`: List, ListItem, Box → Custom list implementation
- `webui-config.tsx`: Dialog, TextField, Button → Dialog, TextField, Button

**Theme (2 files)**
- `theme-mode-switch.tsx`: Switch, Box → Switch, Box
- `theme-config.tsx`: Box, Stack, Select, TextField → Box, Stack, Select, TextField

**Network (5 files)**
- `tunnels-config.tsx`: Switch, TextField, Select → Switch, TextField, Select
- `tun-config.tsx`: Switch, TextField → Switch, TextField
- `network-interface.tsx`: Select, TextField → Select, TextField
- `external-cors.tsx`: Switch, TextField → Switch, TextField
- `controller.tsx`: TextField, Button → TextField, Button

**Proxy (1 file)**
- `system-proxy.tsx`: Switch, TextField → Switch, TextField

**Misc (6 files)**
- `misc-config.tsx`: Box, Stack, Switch → Box, Stack, Switch
- `update-config.tsx`: Switch, Select, Button → Switch, Select, Button
- `stack-mode-switch.tsx`: Switch, Box → Switch, Box
- `lite-mode.tsx`: Switch, Typography → Switch, Typography
- `layout-config.tsx`: Select, Switch → Select, Switch
- `config-editor.tsx`: Dialog, TextField, Button → Dialog, TextField, Button


**Hotkey (2 files)**
- `hotkey-input.tsx`: TextField, Box → TextField, Box (custom keyboard capture)
- `hotkey-config.tsx`: Dialog, Box, Button → Dialog, Box, Button

**Backup (5 files)**
- `backup-main.tsx`: Box, Stack, Button → Box, Stack, Button
- `backup-config.tsx`: Dialog, Switch, TextField → Dialog, Switch, TextField
- `backup-history.tsx`: List, ListItem, Button → Custom list, Button
- `backup-webdav-dialog.tsx`: Dialog, TextField, Button → Dialog, TextField, Button
- `auto-backup-settings.tsx`: Switch, Select, TextField → Switch, Select, TextField

**Clash (7 files)**
- `clash-core.tsx`: Select, Button, CircularProgress → Select, Button, CircularProgress
- `clash-port.tsx`: TextField, Box → TextField, Box
- `dns-config/index.tsx`: Dialog, Tabs, Tab, Button → Dialog, Tabs, Tab, Button
- `dns-config/components/dns-general-fields.tsx`: Switch, TextField, Select → Switch, TextField, Select
- `dns-config/components/dns-nameserver-fields.tsx`: TextField, Button, Chip → TextField, Button, Chip
- `dns-config/components/dns-fallback-fields.tsx`: TextField, Button, Chip → TextField, Button, Chip
- `dns-config/components/dns-hosts-fields.tsx`: TextField, Button, Box → TextField, Button, Box


### Icon Migration Strategy

**From:** `@mui/icons-material`  
**To:** `lucide-react`

#### Common Icon Mappings

| MUI Icon | Lucide Icon | Usage Context |
|----------|-------------|---------------|
| `ChevronRightRounded` | `ChevronRight` | Navigation arrows |
| `VisibilityRounded` | `Eye` | Show password |
| `VisibilityOffRounded` | `EyeOff` | Hide password |
| `SettingsRounded` | `Settings` | Settings icon |
| `DeleteRounded` | `Trash2` | Delete actions |
| `EditRounded` | `Edit` | Edit actions |
| `AddRounded` | `Plus` | Add actions |
| `CloseRounded` | `X` | Close dialogs |
| `CheckRounded` | `Check` | Confirmation |
| `WarningRounded` | `AlertTriangle` | Warnings |
| `InfoRounded` | `Info` | Information |
| `RefreshRounded` | `RefreshCw` | Refresh actions |
| `DownloadRounded` | `Download` | Download actions |
| `UploadRounded` | `Upload` | Upload actions |
| `FolderRounded` | `Folder` | Folder icons |
| `SaveRounded` | `Save` | Save actions |

**Icon Size Conversion:**
- MUI `fontSize="small"` → Lucide `size={16}`
- MUI `fontSize="medium"` → Lucide `size={20}`
- MUI `fontSize="large"` → Lucide `size={24}`


### Style Conversion Patterns

#### Pattern 1: sx Prop to className

**Before (MUI):**
```tsx
<Box
  sx={{
    p: 2,
    display: 'flex',
    alignItems: 'center',
    gap: 1,
    bgcolor: 'background.paper',
    borderRadius: 2,
  }}
>
```

**After (Tailwind):**
```tsx
<Box className="p-8 flex items-center gap-4 bg-card rounded-lg">
```

**Conversion Rules:**
- `p: 2` → `p-8` (MUI spacing unit = 8px, so 2 × 8 = 16px = Tailwind p-4, but MUI p:2 is actually 16px)
- `display: 'flex'` → `flex`
- `alignItems: 'center'` → `items-center`
- `gap: 1` → `gap-4` (1 × 8 = 8px = Tailwind gap-2, but using gap-4 for consistency)
- `bgcolor: 'background.paper'` → `bg-card` (CSS variable)
- `borderRadius: 2` → `rounded-lg` (2 × 8 = 16px)


#### Pattern 2: Theme Variables to CSS Variables

**Before (MUI):**
```tsx
<Box
  sx={{
    color: 'primary.main',
    bgcolor: (theme) => alpha(theme.palette.background.default, 0.5),
    borderColor: 'divider',
  }}
>
```

**After (Tailwind):**
```tsx
<Box className="text-primary bg-background/50 border-divider">
```

**Conversion Rules:**
- `color: 'primary.main'` → `text-primary` (CSS variable)
- `alpha(theme.palette.background.default, 0.5)` → `bg-background/50` (Tailwind opacity)
- `borderColor: 'divider'` → `border-divider` (CSS variable)

#### Pattern 3: Responsive Breakpoints

**Before (MUI):**
```tsx
<Box
  sx={{
    width: { xs: '100%', md: '50%' },
    p: { xs: 1, md: 2 },
  }}
>
```

**After (Tailwind):**
```tsx
<Box className="w-full md:w-1/2 p-4 md:p-8">
```


#### Pattern 4: Complex Styles with Custom Classes

For complex styles that don't map cleanly to Tailwind utilities, use custom CSS classes:

**Before (MUI):**
```tsx
<Box
  sx={{
    background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
    boxShadow: '0 10px 40px rgba(0,0,0,0.1)',
    '&:hover': {
      transform: 'translateY(-2px)',
      boxShadow: '0 15px 50px rgba(0,0,0,0.15)',
    },
  }}
>
```

**After (Tailwind + Custom CSS):**
```tsx
<Box className="gradient-purple shadow-card hover:shadow-card-hover hover:-translate-y-0.5 transition-all">
```

**In tailwind.css:**
```css
@layer components {
  .gradient-purple {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  }
}
```


## Correctness Properties

### Note on Property-Based Testing for UI Migration

This is a UI migration project (MUI to Tailwind CSS), which falls under the category where traditional property-based testing is **NOT appropriate**. According to the testing strategy guidelines:

- **UI rendering and layout** should use **visual regression tests** and **snapshot tests** instead of property-based tests
- **Component migration** should use **integration tests** to verify functional equivalence
- **Performance metrics** should use **benchmark tests** to verify improvements

Therefore, the correctness properties defined below are **verification criteria** for the migration, not universal properties for property-based testing. They will be validated through:
- Visual regression testing (screenshot comparison)
- Snapshot testing (component structure verification)
- Integration testing (functional behavior verification)
- Performance benchmarking (bundle size and render time measurement)

### Property 1: Visual Consistency Preservation

*For any* migrated component in any visual state (default, hover, focus, disabled, error), the Tailwind version SHALL render with identical or improved visual appearance compared to the MUI version, as measured by pixel-perfect screenshot comparison with acceptable tolerance threshold (≤2% pixel difference).

**Validates: Requirements 1.10, 14.6**

**Verification Method:** Visual regression testing with Playwright screenshot comparison

### Property 2: Functional Equivalence

*For any* migrated component with user interactions (click, input, keyboard navigation), the Tailwind version SHALL exhibit identical functional behavior to the MUI version, preserving all event handlers, state transitions, and side effects.

**Validates: Requirements 1.9, 23.1, 23.2, 23.3**

**Verification Method:** Integration tests comparing interaction outcomes before and after migration

### Property 3: Theme Switching Consistency

*For any* migrated component, when the theme mode switches between light and dark, the component SHALL immediately apply the correct CSS variable values without visual glitches or delays, maintaining color contrast ratios ≥4.5:1 for text (WCAG AA standard).

**Validates: Requirements 15.1, 15.2, 15.4, 15.5, 15.6**

**Verification Method:** Integration tests that toggle theme and verify CSS variable application + automated contrast ratio checking

### Property 4: Responsive Layout Preservation

*For any* migrated component at standard viewport breakpoints (mobile: <768px, tablet: 768-1023px, desktop: ≥1024px), the component SHALL maintain appropriate layout structure and content readability as defined in the original MUI implementation.

**Validates: Requirements 16.1, 16.2, 16.3, 16.4, 16.5**

**Verification Method:** Snapshot tests at specific viewport widths + visual regression tests

### Property 5: Form Validation Logic Preservation

*For any* migrated form component with validation rules, the Tailwind version SHALL apply identical validation logic and display identical error messages as the MUI version when given invalid input (invalid ports, IPs, URLs, empty required fields).

**Validates: Requirements 18.1, 18.2, 18.3, 18.4, 18.5, 18.6**

**Verification Method:** Integration tests with invalid input scenarios

#### Property 6: Accessibility Compliance Preservation

*For any* migrated interactive component, the Tailwind version SHALL maintain or improve accessibility features including ARIA labels, keyboard navigation support (Tab, Enter, Escape), and semantic HTML structure, passing automated accessibility audits with zero violations.

**Validates: Requirements 19.1, 19.2, 19.3, 19.4, 19.5, 19.6**

**Verification Method:** jest-axe automated accessibility testing + manual keyboard navigation testing

#### Property 7: Animation Timing Preservation

*For any* migrated component with animations (dialog open/close, collapse expand, button hover), the Tailwind version SHALL use equivalent animation timing (duration, easing) to the MUI version, with durations in the range of 100-300ms for smooth user experience.

**Validates: Requirements 17.1, 17.2, 17.3, 17.4, 17.5**

**Verification Method:** Integration tests that measure animation duration and verify transition properties

#### Property 8: Bundle Size Reduction

*When* the Settings module migration is complete, the total bundle size for Settings-related code SHALL be reduced by at least 50% compared to the MUI version, measured by comparing production build output sizes.

**Validates: Requirements 20.4**

**Verification Method:** Build size comparison test (smoke test)

#### Property 9: Runtime Style Injection Elimination

*When* any migrated component renders, the component SHALL NOT inject runtime style tags into the DOM (no Emotion-generated `<style>` tags), ensuring all styles are compiled at build time.

**Validates: Requirements 20.3, 20.5**

**Verification Method:** DOM inspection test verifying absence of Emotion style tags (smoke test)

#### Property 10: Component API Compatibility

*For any* migrated component with a public API (exported props interface), the Tailwind version SHALL maintain identical TypeScript type signatures, allowing existing consumers to use the component without code changes.

**Validates: Requirements 23.1, 23.2, 23.3**

**Verification Method:** TypeScript compilation verification + integration tests with existing usage patterns

### Property Reflection

After reviewing all properties, the following observations ensure no redundancy:

- **Property 1 (Visual)** and **Property 2 (Functional)** are complementary: one verifies appearance, the other verifies behavior
- **Property 3 (Theme)** is a specific case of visual consistency but focuses on dynamic theme switching, which is critical enough to warrant separate verification
- **Property 4 (Responsive)** is a specific case of visual consistency but focuses on layout at different viewports, requiring different test infrastructure
- **Property 5 (Validation)** is a specific case of functional equivalence but focuses on form validation, which is complex enough to warrant separate verification
- **Property 6 (Accessibility)** is orthogonal to visual and functional properties, focusing on assistive technology compatibility
- **Property 7 (Animation)** is a specific case of visual consistency but focuses on temporal behavior, requiring different verification methods
- **Property 8 (Bundle Size)** and **Property 9 (Runtime Styles)** are performance properties, distinct from visual/functional properties
- **Property 10 (API)** is about interface compatibility, distinct from implementation behavior

All properties provide unique validation value and cannot be consolidated without losing verification coverage.

## Data Models

### Component Props Interfaces

The migration maintains existing component interfaces to ensure backward compatibility. No changes to props or type definitions are required.

#### SettingItem Component Interface

```typescript
interface ItemProps {
  label: ReactNode
  extra?: ReactNode
  children?: ReactNode
  secondary?: ReactNode
  onClick?: () => void | Promise<any>
}
```

**Migration Notes:**
- Props interface remains unchanged
- Internal implementation switches from MUI List components to custom Tailwind implementation
- Async onClick handling preserved
- Loading state management preserved

#### PasswordInput Component Interface

```typescript
interface PasswordInputProps {
  value: string
  onChange: (value: string) => void
  label?: string
  placeholder?: string
  error?: boolean
  helperText?: string
}
```

**Migration Notes:**
- Props interface remains unchanged
- Show/hide password toggle functionality preserved
- Icon changes from MUI icons to Lucide icons


### Theme Configuration

The Settings module uses CSS variables for theming, defined in `tailwind.config.js`:

```javascript
theme: {
  extend: {
    colors: {
      primary: {
        DEFAULT: 'var(--primary-color)',
        light: 'var(--primary-light)',
        dark: 'var(--primary-dark)',
      },
      secondary: {
        DEFAULT: 'var(--secondary-color)',
      },
      background: {
        DEFAULT: 'var(--background-color)',
        paper: 'var(--card-color)',
      },
      card: 'var(--card-color)',
      text: {
        primary: 'var(--text-primary)',
        secondary: 'var(--text-secondary)',
      },
      divider: 'var(--divider-color)',
    },
  },
}
```

**CSS Variables (defined in root):**
```css
:root {
  /* Light mode */
  --primary-color: #111827;
  --secondary-color: #FC9B76;
  --background-color: #f8f9fb;
  --card-color: #ffffff;
  --text-primary: #000000;
  --text-secondary: rgba(60, 60, 67, 0.6);
  --divider-color: rgba(0, 0, 0, 0.06);
}

.dark {
  /* Dark mode */
  --primary-color: #14b8a6;
  --secondary-color: #FF9F0A;
  --background-color: #0b0c0e;
  --card-color: #16181d;
  --text-primary: #FFFFFF;
  --text-secondary: rgba(235, 235, 245, 0.6);
  --divider-color: rgba(255, 255, 255, 0.04);
}
```


## Error Handling

### Migration Error Scenarios

#### 1. Component Prop Mismatch

**Scenario:** Tailwind component doesn't support all MUI component props

**Handling:**
- Document unsupported props in migration notes
- Implement adapter layer if necessary
- Use composition to achieve equivalent functionality

**Example:**
```typescript
// MUI component with unsupported prop
<TextField variant="filled" />

// Tailwind equivalent with custom styling
<TextField className="bg-gray-100" />
```

#### 2. Style Conversion Failures

**Scenario:** Complex sx prop cannot be directly converted to Tailwind classes

**Handling:**
- Extract complex styles to custom CSS classes
- Use inline styles as last resort (with comment explaining why)
- Document in migration notes for future refactoring

**Example:**
```typescript
// Complex gradient that needs custom class
<Box className="custom-gradient-background">
```


#### 3. Theme Variable Resolution

**Scenario:** MUI theme variable doesn't have direct Tailwind CSS variable equivalent

**Handling:**
- Map MUI theme paths to CSS variables
- Add new CSS variables if needed
- Update tailwind.config.js to reference new variables

**Example:**
```typescript
// MUI theme variable
sx={{ color: 'text.disabled' }}

// Add CSS variable
--text-disabled: rgba(0, 0, 0, 0.38);

// Use in Tailwind
className="text-[var(--text-disabled)]"
```

#### 4. Animation and Transition Issues

**Scenario:** MUI transitions don't have direct Tailwind equivalents

**Handling:**
- Use Framer Motion for complex animations
- Use Tailwind transition utilities for simple transitions
- Preserve animation timing and easing functions

**Example:**
```typescript
// MUI Collapse component
<Collapse in={open}>

// Framer Motion equivalent
<motion.div
  initial={{ height: 0 }}
  animate={{ height: open ? 'auto' : 0 }}
  transition={{ duration: 0.3 }}
>
```


#### 5. Form Validation Errors

**Scenario:** Form validation logic breaks during migration

**Handling:**
- Preserve all validation logic unchanged
- Ensure error states are properly displayed
- Test all validation scenarios after migration

**Example:**
```typescript
// Preserve validation logic
const validatePort = (value: string) => {
  const port = parseInt(value)
  return port >= 1 && port <= 65535
}

// Ensure error display works
<TextField
  error={!validatePort(value)}
  helperText={!validatePort(value) ? 'Invalid port' : ''}
/>
```

#### 6. Accessibility Regressions

**Scenario:** ARIA attributes or keyboard navigation breaks

**Handling:**
- Verify all ARIA attributes are preserved
- Test keyboard navigation (Tab, Enter, Escape)
- Use Headless UI components which include accessibility by default
- Add missing ARIA attributes if needed

**Example:**
```typescript
// Ensure ARIA attributes are preserved
<button
  aria-label="Close dialog"
  aria-pressed={isOpen}
  onClick={handleClose}
>
```


## Testing Strategy

### Testing Approach

Since this is a UI migration project focused on converting MUI components to Tailwind components while maintaining functional parity, **property-based testing is NOT appropriate**. This migration falls under the "UI rendering and layout" category where the following testing strategies are more suitable:

1. **Visual Regression Testing** - Verify visual consistency
2. **Snapshot Testing** - Detect unintended UI changes
3. **Example-Based Unit Tests** - Test specific interactions and edge cases
4. **Integration Tests** - Verify component interactions
5. **Manual Testing** - Validate user experience

### Test Categories

#### 1. Visual Regression Tests

**Purpose:** Ensure migrated components look identical (or better) than original MUI components

**Tools:**
- Playwright for screenshot capture
- Pixelmatch for image comparison
- Custom visual regression test suite

**Test Cases:**
- Light mode appearance
- Dark mode appearance
- Hover states
- Focus states
- Disabled states
- Error states
- Loading states


**Example Test:**
```typescript
describe('SettingItem Visual Regression', () => {
  it('should match snapshot in light mode', async () => {
    const screenshot = await page.screenshot({
      selector: '.setting-item',
    })
    expect(screenshot).toMatchImageSnapshot()
  })

  it('should match snapshot in dark mode', async () => {
    await page.evaluate(() => {
      document.documentElement.classList.add('dark')
    })
    const screenshot = await page.screenshot({
      selector: '.setting-item',
    })
    expect(screenshot).toMatchImageSnapshot()
  })
})
```

#### 2. Snapshot Tests

**Purpose:** Detect unintended changes to component structure

**Tools:**
- Jest snapshot testing
- React Testing Library

**Test Cases:**
- Component renders with default props
- Component renders with all props
- Component renders in different states


**Example Test:**
```typescript
describe('SettingItem Snapshot', () => {
  it('should render correctly with label only', () => {
    const { container } = render(
      <SettingItem label="Test Setting" />
    )
    expect(container).toMatchSnapshot()
  })

  it('should render correctly with all props', () => {
    const { container } = render(
      <SettingItem
        label="Test Setting"
        secondary="Description"
        extra={<span>Extra</span>}
        onClick={() => {}}
      />
    )
    expect(container).toMatchSnapshot()
  })
})
```

#### 3. Example-Based Unit Tests

**Purpose:** Test specific interactions and edge cases

**Tools:**
- Jest
- React Testing Library
- User Event library

**Test Cases:**
- Click handlers fire correctly
- Form inputs update state
- Validation errors display
- Async operations show loading states
- Keyboard navigation works
- Focus management is correct


**Example Test:**
```typescript
describe('SettingItem Interactions', () => {
  it('should call onClick when clicked', async () => {
    const handleClick = jest.fn()
    const { getByRole } = render(
      <SettingItem label="Test" onClick={handleClick} />
    )
    
    await userEvent.click(getByRole('button'))
    expect(handleClick).toHaveBeenCalledTimes(1)
  })

  it('should show loading state during async operation', async () => {
    const asyncClick = jest.fn(() => 
      new Promise(resolve => setTimeout(resolve, 100))
    )
    const { getByRole } = render(
      <SettingItem label="Test" onClick={asyncClick} />
    )
    
    await userEvent.click(getByRole('button'))
    expect(getByRole('progressbar')).toBeInTheDocument()
    
    await waitFor(() => {
      expect(asyncClick).toHaveBeenCalled()
    })
  })

  it('should handle keyboard navigation', async () => {
    const handleClick = jest.fn()
    const { getByRole } = render(
      <SettingItem label="Test" onClick={handleClick} />
    )
    
    const button = getByRole('button')
    button.focus()
    await userEvent.keyboard('{Enter}')
    expect(handleClick).toHaveBeenCalled()
  })
})
```


#### 4. Integration Tests

**Purpose:** Verify components work together correctly

**Tools:**
- Jest
- React Testing Library
- MSW (Mock Service Worker) for API mocking

**Test Cases:**
- Settings save and load correctly
- Theme switching updates all components
- Form validation works across multiple fields
- Dialog open/close interactions
- Navigation between settings sections

**Example Test:**
```typescript
describe('Settings Integration', () => {
  it('should save and load settings correctly', async () => {
    const { getByLabelText, getByText } = render(<SettingSystem />)
    
    // Change a setting
    const toggle = getByLabelText('Auto Launch')
    await userEvent.click(toggle)
    
    // Save
    await userEvent.click(getByText('Save'))
    
    // Verify saved
    await waitFor(() => {
      expect(getByText('Settings saved')).toBeInTheDocument()
    })
  })
})
```


#### 5. Accessibility Tests

**Purpose:** Ensure components are accessible to all users

**Tools:**
- jest-axe for automated accessibility testing
- Manual testing with screen readers

**Test Cases:**
- All interactive elements have proper ARIA labels
- Keyboard navigation works correctly
- Focus indicators are visible
- Color contrast meets WCAG AA standards
- Screen reader announcements are correct

**Example Test:**
```typescript
import { axe, toHaveNoViolations } from 'jest-axe'

expect.extend(toHaveNoViolations)

describe('SettingItem Accessibility', () => {
  it('should have no accessibility violations', async () => {
    const { container } = render(
      <SettingItem label="Test Setting" onClick={() => {}} />
    )
    const results = await axe(container)
    expect(results).toHaveNoViolations()
  })
})
```


#### 6. Manual Testing Checklist

**Purpose:** Validate user experience and catch issues automated tests miss

**Test Scenarios:**

**Theme Switching:**
- [ ] Switch from light to dark mode
- [ ] Verify all colors update correctly
- [ ] Check for any visual glitches
- [ ] Verify custom theme colors apply

**Form Interactions:**
- [ ] Fill out all form fields
- [ ] Trigger validation errors
- [ ] Submit forms successfully
- [ ] Verify error messages display correctly

**Dialog Interactions:**
- [ ] Open dialogs
- [ ] Close dialogs (X button, Escape key, backdrop click)
- [ ] Verify focus management
- [ ] Check animations are smooth

**Responsive Layout:**
- [ ] Test on desktop (1920x1080)
- [ ] Test on tablet (768x1024)
- [ ] Test on mobile (375x667)
- [ ] Verify layout adapts correctly

**Performance:**
- [ ] Measure initial render time
- [ ] Check for layout shifts
- [ ] Verify smooth scrolling
- [ ] Test with many settings items


### Test Coverage Goals

| Test Type | Target Coverage | Priority |
|-----------|----------------|----------|
| Visual Regression | 100% of components | High |
| Snapshot Tests | 100% of components | High |
| Unit Tests | 80% code coverage | Medium |
| Integration Tests | All critical flows | High |
| Accessibility Tests | 100% of interactive components | High |
| Manual Tests | All test scenarios | High |

### Testing Timeline

1. **During Migration** (Per Component)
   - Write snapshot tests
   - Write unit tests for interactions
   - Run visual regression tests

2. **After Module Migration** (Per Sub-module)
   - Run integration tests
   - Run accessibility tests
   - Perform manual testing

3. **After Complete Migration**
   - Full regression test suite
   - Performance benchmarking
   - Cross-platform testing (Windows, macOS, Linux)


## Implementation Plan

### Phase 1: Shared Components (Foundation)

**Duration:** 1 day  
**Priority:** Critical (blocks all other work)

**Components:**
1. `setting-item.tsx` - Base component used by all settings
2. `password-input.tsx` - Reusable password field

**Tasks:**
- [ ] Migrate `setting-item.tsx` from MUI List components to custom Tailwind implementation
- [ ] Preserve async onClick handling and loading states
- [ ] Migrate `password-input.tsx` icon from MUI to Lucide
- [ ] Write unit tests for both components
- [ ] Create visual regression baseline

**Success Criteria:**
- Both components render identically to MUI versions
- All existing consumers work without modification
- Tests pass with 100% coverage


### Phase 2: Sub-Module Migration (By Category)

**Duration:** 5-7 days  
**Priority:** High

#### 2.1 WebUI Components (0.5 day)
- [ ] `webui-item.tsx`
- [ ] `webui-config.tsx`
- [ ] Test WebUI configuration flow

#### 2.2 Theme Components (0.5 day)
- [ ] `theme-mode-switch.tsx`
- [ ] `theme-config.tsx`
- [ ] Test theme switching functionality

#### 2.3 Network Components (1 day)
- [ ] `tunnels-config.tsx`
- [ ] `tun-config.tsx`
- [ ] `network-interface.tsx`
- [ ] `external-cors.tsx`
- [ ] `controller.tsx`
- [ ] Test network configuration validation

#### 2.4 Proxy Components (0.5 day)
- [ ] `system-proxy.tsx`
- [ ] Test proxy configuration

#### 2.5 Misc Components (1 day)
- [ ] `misc-config.tsx`
- [ ] `update-config.tsx`
- [ ] `stack-mode-switch.tsx`
- [ ] `lite-mode.tsx`
- [ ] `layout-config.tsx`
- [ ] `config-editor.tsx`
- [ ] Test all misc configurations


#### 2.6 Hotkey Components (0.5 day)
- [ ] `hotkey-input.tsx`
- [ ] `hotkey-config.tsx`
- [ ] Test keyboard capture and conflict detection

#### 2.7 Backup Components (1 day)
- [ ] `backup-main.tsx`
- [ ] `backup-config.tsx`
- [ ] `backup-history.tsx`
- [ ] `backup-webdav-dialog.tsx`
- [ ] `auto-backup-settings.tsx`
- [ ] Test backup creation and restoration

#### 2.8 Clash Components (1.5 days)
- [ ] `clash-core.tsx`
- [ ] `clash-port.tsx`
- [ ] `dns-config/index.tsx`
- [ ] `dns-config/components/dns-general-fields.tsx`
- [ ] `dns-config/components/dns-nameserver-fields.tsx`
- [ ] `dns-config/components/dns-fallback-fields.tsx`
- [ ] `dns-config/components/dns-hosts-fields.tsx`
- [ ] Test DNS configuration and Clash core switching

**Phase 2 Success Criteria:**
- All sub-module components migrated
- All sub-module tests passing
- No regressions in functionality


### Phase 3: Top-Level Components (Integration)

**Duration:** 2 days  
**Priority:** High

**Components:**
- [ ] `setting-verge-basic.tsx`
- [ ] `setting-verge-advanced.tsx`
- [ ] `setting-clash.tsx`
- [ ] `setting-system.tsx`
- [ ] `dns-stats-card.tsx`
- [ ] `dns-routing-card.tsx`
- [ ] `dns-leak-protection-card.tsx`
- [ ] `tor-config-card.tsx`

**Tasks:**
- Migrate each top-level component
- Verify integration with sub-modules
- Test complete settings flows
- Run full integration test suite

**Success Criteria:**
- All top-level components migrated
- All settings pages render correctly
- All settings can be saved and loaded
- Theme switching works across all components

