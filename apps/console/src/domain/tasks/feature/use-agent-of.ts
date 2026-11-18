import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { shortName } from '#/domain/demo'
import type { AgentView } from '#/domain/tasks/ui/tasks-view'

const UNASSIGNED: AgentView = {
	name: 'Non assignée',
	fullName: 'Non assignée',
	hue: 250,
	model: '',
}

export function useAgentOf(): (agentId: string | null) => AgentView {
	const agents = useTeamAgents().data?.items
	return (agentId) => {
		if (!agentId) return UNASSIGNED
		const agent = agents?.find((a) => a.id === agentId)
		return agent
			? {
					name: shortName(agent.name),
					fullName: agent.name,
					hue: agent.hue,
					model: agent.model,
				}
			: { name: 'Agent', fullName: 'Agent', hue: 250, model: '' }
	}
}
