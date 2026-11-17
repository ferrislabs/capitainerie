import type { LucideIcon } from 'lucide-react'
import {
	Bot,
	Box,
	FolderKanban,
	History,
	Inbox,
	LayoutDashboard,
	ListChecks,
	Plug,
	RadioTower,
	Repeat,
	ShieldCheck,
	Sparkles,
} from 'lucide-react'

export interface NavItem {
	to:
		| '/'
		| '/inbox/{-$tab}'
		| '/tasks'
		| '/projects'
		| '/routines'
		| '/artifacts'
		| '/agents'
		| '/skills'
		| '/sources'
		| '/connectors/{-$tab}'
		| '/approvals'
		| '/audit/{-$tab}'
	label: string
	icon: LucideIcon
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
			{ to: '/', label: 'Tableau de bord', icon: LayoutDashboard },
			{ to: '/inbox/{-$tab}', label: 'Boîte de réception', icon: Inbox },
		],
	},
	{
		label: 'Travail',
		items: [
			{ to: '/tasks', label: 'Tâches', icon: ListChecks },
			{ to: '/projects', label: 'Projets', icon: FolderKanban },
			{ to: '/routines', label: 'Routines', icon: Repeat },
			{ to: '/artifacts', label: 'Artifacts', icon: Box },
		],
	},
	{
		label: 'Organisation',
		items: [
			{ to: '/agents', label: 'Agents', icon: Bot },
			{ to: '/skills', label: 'Compétences', icon: Sparkles },
			{ to: '/sources', label: 'Sources', icon: RadioTower },
			{ to: '/connectors/{-$tab}', label: 'Connecteurs', icon: Plug },
			{ to: '/approvals', label: 'Approbations', icon: ShieldCheck },
			{ to: '/audit/{-$tab}', label: 'Audit', icon: History },
		],
	},
]
