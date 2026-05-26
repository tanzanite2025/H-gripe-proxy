import { createBrowserRouter, RouteObject } from 'react-router'

import Layout from './_layout'
import ConnectionsPage from './connections'
import HomePage from './home'
import ProfilesPage from './profiles'
import ProxiesPage from './proxies'
import RulesPage from './rules'
import SettingsPage from './settings'
import UnlockPage from './unlock'

export const navItems = [
  {
    label: 'layout.components.navigation.tabs.home',
    path: '/',
    icon: [],
    Component: HomePage,
  },
  {
    label: 'layout.components.navigation.tabs.proxies',
    path: '/proxies',
    icon: [],
    Component: ProxiesPage,
  },
  {
    label: 'layout.components.navigation.tabs.profiles',
    path: '/profile',
    icon: [],
    Component: ProfilesPage,
  },
  {
    label: 'layout.components.navigation.tabs.connections',
    path: '/connections',
    icon: [],
    Component: ConnectionsPage,
  },
  {
    label: 'layout.components.navigation.tabs.rules',
    path: '/rules',
    icon: [],
    Component: RulesPage,
  },
  {
    label: 'layout.components.navigation.tabs.logs',
    path: '/logs',
    icon: [],
    Component: () => null /* KeepAlive: real LogsPage rendered in Layout */,
  },
  {
    label: 'layout.components.navigation.tabs.unlock',
    path: '/unlock',
    icon: [],
    Component: UnlockPage,
  },
  {
    label: 'layout.components.navigation.tabs.settings',
    path: '/settings',
    icon: [],
    Component: SettingsPage,
  },
]

export const router = createBrowserRouter([
  {
    path: '/',
    Component: Layout,
    children: navItems.map(
      (item) =>
        ({
          path: item.path,
          Component: item.Component,
        }) as RouteObject,
    ),
  },
])
