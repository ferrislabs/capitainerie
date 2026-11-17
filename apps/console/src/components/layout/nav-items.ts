import type { LucideIcon } from 'lucide-react'
import {
	Bot,
	Box,
	Cpu,
	FolderKanban,
	Github,
	History,
	House,
	Inbox,
	KeyRound,
	MessageSquare,
	Play,
	Plug,
	Radio,
	RadioTower,
	Repeat,
	ShieldCheck,
	Sparkles,
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
		| '/routines'
		| '/inbox/{-$tab}'
		| '/artifacts'
		| '/agents'
		| '/skills'
		| '/sources'
		| '/connectors/{-$tab}'
		| '/approvals'
		| '/audit/{-$tab}'
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
			{ to: '/inbox/{-$tab}', label: 'Boîte de réception', icon: Inbox },
			{ to: '/chat', label: 'Conversations', icon: MessageSquare },
			{ to: '/runs', label: 'Runs', icon: Play },
			{ to: '/kanban', label: 'Kanban', icon: SquareKanban },
			{ to: '/clients', label: 'Clients', icon: Users },
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
		],
	},
	{
		label: 'Supervision',
		items: [
			{ to: '/live', label: 'En cours', icon: Radio },
			{ to: '/audit/{-$tab}', label: 'Audit', icon: History },
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
