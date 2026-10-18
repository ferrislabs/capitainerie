import type { LucideIcon } from 'lucide-react'
import { Bot, Cpu, Github, KeyRound, Play, SquareKanban } from 'lucide-react'

export interface NavItem {
	to:
		| '/runs'
		| '/kanban'
		| '/agents'
		| '/settings/providers'
		| '/settings/models'
		| '/settings/integrations'
	label: string
	icon: LucideIcon
}

export interface NavSection {
	label: string
	items: ReadonlyArray<NavItem>
}

export const NAV_SECTIONS: ReadonlyArray<NavSection> = [
	{
		label: 'Travail',
		items: [
			{ to: '/runs', label: 'Runs', icon: Play },
			{ to: '/kanban', label: 'Kanban', icon: SquareKanban },
		],
	},
	{
		label: 'Supervision',
		items: [{ to: '/agents', label: 'Agents', icon: Bot }],
	},
	{
		label: 'Paramètres',
		items: [
			{ to: '/settings/providers', label: 'Fournisseurs', icon: KeyRound },
			{ to: '/settings/models', label: 'Modèles', icon: Cpu },
			{ to: '/settings/integrations', label: 'Intégrations', icon: Github },
		],
	},
]
