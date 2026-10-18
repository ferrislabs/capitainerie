import type { LucideIcon } from 'lucide-react'
import { Bot, Play, Settings2, SquareKanban } from 'lucide-react'

export interface NavItem {
	to: '/runs' | '/kanban' | '/agents' | '/settings'
	label: string
	icon: LucideIcon
}

export const PRIMARY_NAV: ReadonlyArray<NavItem> = [
	{ to: '/runs', label: 'Runs', icon: Play },
	{ to: '/kanban', label: 'Kanban', icon: SquareKanban },
	{ to: '/agents', label: 'Agents', icon: Bot },
]

export const SECONDARY_NAV: ReadonlyArray<NavItem> = [
	{ to: '/settings', label: 'Paramètres', icon: Settings2 },
]
