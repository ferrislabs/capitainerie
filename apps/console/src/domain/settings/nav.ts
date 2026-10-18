import { Cpu, KeyRound } from 'lucide-react'
import type { SettingsNavItem } from '#/components/layout/settings-nav'

export const SETTINGS_NAV: ReadonlyArray<SettingsNavItem> = [
	{ to: '/settings/providers', label: 'Fournisseurs', icon: KeyRound },
	{ to: '/settings/models', label: 'Modèles', icon: Cpu },
]
