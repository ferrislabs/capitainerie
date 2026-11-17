import { createFileRoute } from '@tanstack/react-router'
import { AgentTeamPage } from '#/domain/agent-team/feature/agent-team-page'

export const Route = createFileRoute('/_app/agents/')({
	component: AgentTeamPage,
})
