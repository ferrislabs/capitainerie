import { createFileRoute } from '@tanstack/react-router'
import {
	type AgentFilter,
	parseAgentFilter,
} from '#/domain/agent-team/agent-filters'
import { AgentTeamPage } from '#/domain/agent-team/feature/agent-team-page'

export const Route = createFileRoute('/_app/agents/')({
	validateSearch: (
		search: Record<string, unknown>,
	): { filtre?: AgentFilter } => ({
		filtre: parseAgentFilter(search.filtre),
	}),
	component: AgentsRoute,
})

function AgentsRoute() {
	const { filtre } = Route.useSearch()
	return <AgentTeamPage filter={filtre} />
}
