import { createFileRoute } from '@tanstack/react-router'
import { AgentDetailPage } from '#/domain/agent-team/feature/agent-detail-page'
import type { InstructionsMode } from '#/domain/agent-team/ui/instructions-tab'

const MODES: ReadonlyArray<InstructionsMode> = ['lire', 'editer', 'brut']

export const Route = createFileRoute('/_app/agents/equipe_/$agentId/{-$tab}')({
	validateSearch: (
		search: Record<string, unknown>,
	): { mode?: InstructionsMode } => {
		const mode = MODES.find((m) => m === search.mode)
		return mode ? { mode } : {}
	},
	component: AgentDetailRoute,
})

function AgentDetailRoute() {
	const { agentId, tab } = Route.useParams()
	const { mode } = Route.useSearch()
	return <AgentDetailPage agentId={agentId} tab={tab} mode={mode} />
}
