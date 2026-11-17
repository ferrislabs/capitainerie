import { Link } from '@tanstack/react-router'
import {
	AGENT_FILTERS,
	type AgentFilter,
} from '#/domain/agent-team/agent-filters'
import { cn } from '#/lib/utils'

export type AgentsTab = AgentFilter | 'organigramme'

interface Props {
	active: AgentsTab
}

const TAB =
	'-mb-px inline-block whitespace-nowrap border-b-2 border-transparent px-2.5 py-[7px] font-medium text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring'
const ON = 'border-primary text-foreground'

export function AgentsTabs({ active }: Props) {
	const mark = (on: boolean) => {
		const props = { 'aria-current': on ? ('page' as const) : undefined }
		return { activeProps: props, inactiveProps: props }
	}
	return (
		<nav
			aria-label="Sections des agents"
			className="mb-3.5 flex gap-0.5 overflow-x-auto border-b"
		>
			{AGENT_FILTERS.map((filter) => (
				<Link
					key={filter.id}
					to="/agents"
					search={{ filtre: filter.id }}
					{...mark(active === filter.id)}
					className={cn(TAB, active === filter.id && ON)}
				>
					{filter.label}
				</Link>
			))}
			<Link
				to="/agents/organigramme"
				{...mark(active === 'organigramme')}
				className={cn(TAB, active === 'organigramme' && ON)}
			>
				Organigramme
			</Link>
		</nav>
	)
}
