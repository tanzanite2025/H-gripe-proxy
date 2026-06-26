import { createBrowserRouter, RouteObject } from 'react-router'

import Layout from '../_layout/layout'
import AdvancedPage from '../advanced'
import ConnectionsPage from '../connections'
import DnsPage from '../dns'
import HomePage from '../home'
import NetworkDiagnosticPage from '../network-diagnostic'
import ProfilesPage from '../profiles'
import ProxiesPage from '../proxies'
import SecurityPage from '../security'
import SettingsPage from '../settings'

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
    label: 'layout.components.navigation.tabs.logs',
    path: '/logs',
    icon: [],
    Component: () => null /* KeepAlive: real LogsPage rendered in Layout */,
  },
  {
    label: 'layout.components.navigation.tabs.networkDiagnostic',
    path: '/network-diagnostic',
    icon: [],
    Component: NetworkDiagnosticPage,
  },
  {
    label: 'layout.components.navigation.tabs.dns',
    path: '/dns',
    icon: [],
    Component: DnsPage,
  },
  {
    label: 'layout.components.navigation.tabs.security',
    path: '/security',
    icon: [],
    Component: SecurityPage,
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
    children: [
      ...navItems.map(
        (item) =>
          ({
            path: item.path,
            Component: item.Component,
          }) as RouteObject,
      ),
      {
        path: '/advanced',
        Component: AdvancedPage,
      },
    ],
  },
])
