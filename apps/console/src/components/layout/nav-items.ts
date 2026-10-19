import type { LucideIcon } from 'lucide-react'
import {
	Bot,
	Cpu,
	FolderKanban,
	Github,
	House,
	KeyRound,
	MessageSquare,
	Play,
	Radio,
	SquareKanban,
	Users,
} from 'lucide-react'

export interface NavItem {
	to:
		| '/'
		| '/chat'
		| '/runs'
		| '/live'
		| '/kanban'
		| '/clients'
		| '/projects'
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
			{ to: '/', label: 'Accueil', icon: House },
			{ to: '/chat', label: 'Conversations', icon: MessageSquare },
			{ to: '/runs', label: 'Runs', icon: Play },
			{ to: '/kanban', label: 'Kanban', icon: SquareKanban },
			{ to: '/clients', label: 'Clients', icon: Users },
			{ to: '/projects', label: 'Projets', icon: FolderKanban },
		],
	},
	{
		label: 'Supervision',
		items: [
			{ to: '/live', label: 'En cours', icon: Radio },
			{ to: '/agents', label: 'Agents', icon: Bot },
		],
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
