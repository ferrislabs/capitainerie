import { Bot } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import {
	getAgentConfig,
	useConfigVersion,
} from '#/domain/agent-team/agent-config-store'
import {
	AGENT_FILTERS,
	type AgentFilter,
} from '#/domain/agent-team/agent-filters'
import { AgentList } from '#/domain/agent-team/ui/agent-list'
import { AgentsHeader } from '#/domain/agent-team/ui/agents-header'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'
import { Panel } from '#/domain/agent-team/ui/panel'
import { Empty } from '#/domain/agent-team/ui/rows'
import { type DemoAgent, useDemoState } from '#/domain/demo'

interface Props {
	filter?: AgentFilter
}

function visible(agents: ReadonlyArray<DemoAgent>, filter: AgentFilter) {
	if (filter === 'tous') return agents
	if (filter === 'actifs') return agents.filter((a) => a.status === 'running')
	if (filter === 'pause')
		return agents.filter((a) => getAgentConfig(a.id).paused)
	return []
}

export function AgentTeamPage({ filter = 'tous' }: Props) {
	const agents = useDemoState((s) => s.agents)
	useConfigVersion()
	const [message, setMessage] = useNotice()
	const list = visible(agents, filter)
	const label = AGENT_FILTERS.find((f) => f.id === filter)?.label ?? ''
	return (
		<Page>
			<AgentsHeader />
			<DemoBanner />
			<AgentsTabs active={filter} />
			<Notice message={message} />
			<Panel>
				{list.length > 0 ? (
					<>
						<AgentList agents={list} />
						<div className="flex min-h-9 items-center justify-center border-t px-3 py-1.5">
							<button
								type="button"
								className="text-xs text-primary"
								onClick={() => setMessage('13 agents supplémentaires')}
							>
								Afficher les 13 autres agents
							</button>
						</div>
					</>
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
