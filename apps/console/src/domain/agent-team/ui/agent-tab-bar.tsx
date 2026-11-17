import { Link } from '@tanstack/react-router'
import { AGENT_TABS, type AgentTabId } from '#/domain/agent-team/agent-tabs'
import { cn } from '#/lib/utils'

interface Props {
	agentId: string
	active: AgentTabId
}

const TAB_CLASS =
	'-mb-px whitespace-nowrap border-b-2 border-transparent px-2.5 py-1.5 font-medium text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring'

export function AgentTabBar({ agentId, active }: Props) {
	return (
		<nav
			aria-label="Sections de l'agent"
			className="flex items-center gap-0.5 overflow-x-auto border-b"
		>
			{AGENT_TABS.map((tab) => (
				<Link
					key={tab.id}
					to="/agents/equipe/$agentId/{-$tab}"
					params={{ agentId, tab: tab.id }}
					aria-current={tab.id === active ? 'page' : undefined}
					className={cn(
						TAB_CLASS,
						tab.id === active && 'border-primary text-foreground',
					)}
				>
					{tab.label}
				</Link>
			))}
			<span className="flex-1" />
			<Link to="/runs" className={TAB_CLASS}>
				Exécutions
			</Link>
			<Link to="/agents" className={TAB_CLASS}>
				Coûts
			</Link>
		</nav>
	)
}
