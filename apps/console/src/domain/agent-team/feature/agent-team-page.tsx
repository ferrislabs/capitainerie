import { Bot } from 'lucide-react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { Skeleton } from '#/components/ui/skeleton'
import {
	AGENT_FILTERS,
	type AgentFilter,
} from '#/domain/agent-team/agent-filters'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { AgentList } from '#/domain/agent-team/ui/agent-list'
import { AgentsHeader } from '#/domain/agent-team/ui/agents-header'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'
import { Panel } from '#/domain/agent-team/ui/panel'
import { Empty } from '#/domain/agent-team/ui/rows'

interface Props {
	filter?: AgentFilter
}

function visible(agents: ReadonlyArray<TeamAgent>, filter: AgentFilter) {
	if (filter === 'tous' || filter === 'archives') return agents
	if (filter === 'actifs') return agents.filter((a) => a.status === 'running')
	if (filter === 'pause') return agents.filter((a) => a.paused)
	return []
}

export function AgentTeamPage({ filter = 'tous' }: Props) {
	const query = useTeamAgents(filter === 'archives')
	const list = visible(query.data?.items ?? [], filter)
	const label = AGENT_FILTERS.find((f) => f.id === filter)?.label ?? ''
	return (
		<Page>
			<AgentsHeader />
			<AgentsTabs active={filter} />
			<Panel>
				{query.isError ? (
					<InlineAlert tone="danger">
						Impossible de charger les agents.
					</InlineAlert>
				) : query.isPending ? (
					<output className="grid gap-2 p-3">
						<span className="sr-only">Chargement…</span>
						{['a', 'b', 'c'].map((key) => (
							<Skeleton key={key} className="h-9" />
						))}
					</output>
				) : list.length > 0 ? (
					<AgentList agents={list} />
				) : (
					<Empty
						icon={<Bot aria-hidden="true" className="mx-auto size-4" />}
						title="Aucun agent"
					>
						Aucun agent {label.toLowerCase()} pour l'instant.
					</Empty>
				)}
			</Panel>
		</Page>
	)
}
