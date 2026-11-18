import { Link } from '@tanstack/react-router'
import { AGENT_TABS, type AgentTabId } from '#/domain/agent-team/agent-tabs'
import { cn } from '#/lib/utils'

interface Props {
	agentId: string
	active: AgentTabId
}

const TAB_CLASS =
	'inline-flex h-11 shrink-0 items-center whitespace-nowrap border-b-2 border-transparent px-2.5 text-[14px] font-medium text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring'

export function AgentTabBar({ agentId, active }: Props) {
	return (
		<nav
			aria-label="Sections de l'agent"
			className="mt-8 flex gap-0.5 overflow-x-auto overflow-y-hidden shadow-[inset_0_-1px_0_var(--color-border)] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
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
			<Link
				to="/audit/{-$tab}"
				params={{ tab: 'runs' }}
				className={cn(TAB_CLASS, 'font-normal')}
			>
				Audit →
			</Link>
		</nav>
	)
}
