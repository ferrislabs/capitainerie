import type { LucideIcon } from 'lucide-react'
import {
	Box,
	ChartColumn,
	CircleCheck,
	Folder,
	HardHat,
	History,
	House,
	Inbox,
	KeyRound,
	Plug,
	Radio,
	Repeat,
	Shield,
	Sparkles,
	Users,
} from 'lucide-react'

export interface NavItem {
	to:
		| '/'
		| '/inbox/{-$tab}'
		| '/tasks'
		| '/projects'
		| '/worksites'
		| '/routines'
		| '/artifacts'
		| '/agents'
		| '/skills'
		| '/sources'
		| '/connectors/{-$tab}'
		| '/approvals'
		| '/audit/{-$tab}'
		| '/metrics/{-$tab}'
		| '/providers'
	label: string
	icon: LucideIcon
	live?: boolean
}

export interface NavSection {
	/** Titre de la section ; vide pour le groupe du haut, sans titre. */
	label: string
	items: ReadonlyArray<NavItem>
}

export const NAV_SECTIONS: ReadonlyArray<NavSection> = [
	{
		label: '',
		items: [
			{ to: '/', label: 'Tableau de bord', icon: House, live: true },
			{ to: '/inbox/{-$tab}', label: 'Boîte de réception', icon: Inbox },
		],
	},
	{
		label: 'Travail',
		items: [
			{ to: '/tasks', label: 'Tâches', icon: CircleCheck },
			{ to: '/projects', label: 'Projets', icon: Folder },
			{ to: '/worksites', label: 'Chantiers', icon: HardHat },
			{ to: '/routines', label: 'Routines', icon: Repeat },
			{ to: '/artifacts', label: 'Artifacts', icon: Box },
		],
	},
	{
		label: 'Organisation',
		items: [
			{ to: '/agents', label: 'Agents', icon: Users },
			{ to: '/skills', label: 'Compétences', icon: Sparkles },
			{ to: '/sources', label: 'Sources', icon: Radio },
			{ to: '/connectors/{-$tab}', label: 'Connecteurs', icon: Plug },
			{ to: '/approvals', label: 'Approbations', icon: Shield },
			{ to: '/audit/{-$tab}', label: 'Audit', icon: History },
			{ to: '/metrics/{-$tab}', label: 'Métriques', icon: ChartColumn },
		],
	},
	{
		label: 'Mon compte',
		items: [{ to: '/providers', label: 'Fournisseurs LLM', icon: KeyRound }],
	},
]
