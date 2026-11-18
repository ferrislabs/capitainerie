import { Link } from '@tanstack/react-router'
import { AGENT_TABS, type AgentTabId } from '#/domain/agent-team/agent-tabs'
import { cn } from '#/lib/utils'

interface Props {
	agentId: string
	active: AgentTabId
}

const TAB_CLASS =
	'-mb-px inline-flex h-11 items-center whitespace-nowrap border-b-2 border-transparent px-3 text-[14px] font-medium text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring'

const AUDIT_LINKS = [
	{ slug: 'activite', label: 'Activité' },
	{ slug: 'runs', label: 'Exécutions' },
	{ slug: 'couts', label: 'Coûts' },
	{ slug: 'budgets', label: 'Budgets' },
] as const

export function AgentTabBar({ agentId, active }: Props) {
	return (
		<nav
			aria-label="Sections de l'agent"
			className="mt-8 flex gap-1 overflow-x-auto border-b"
		>
			{AGENT_TABS.map((tab) => {
				const props = {
					'aria-current': tab.id === active ? ('page' as const) : undefined,
				}
				return (
					<Link
						key={tab.id}
						to="/agents/$agentId/{-$tab}"
						params={{ agentId, tab: tab.id }}
						activeProps={props}
						inactiveProps={props}
						className={cn(
							TAB_CLASS,
							tab.id === active && 'border-primary text-foreground',
						)}
					>
						{tab.label}
					</Link>
				)
			})}
			<span className="min-w-6 flex-1" />
			{AUDIT_LINKS.map((link) => (
				<Link
					key={link.slug}
					to="/audit/{-$tab}"
					params={{ tab: link.slug }}
					className={cn(TAB_CLASS, 'font-normal')}
				>
					{link.label}
				</Link>
			))}
		</nav>
	)
}
