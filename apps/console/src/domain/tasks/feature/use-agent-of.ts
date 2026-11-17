import { shortName, useDemoState } from '#/domain/demo'
import type { AgentView } from '#/domain/tasks/ui/tasks-view'

export function useAgentOf(): (agentId: string) => AgentView {
	const agents = useDemoState((s) => s.agents)
	return (agentId) => {
		const agent = agents.find((a) => a.id === agentId)
		return agent
			? {
					name: shortName(agent.name),
					fullName: agent.name,
					hue: agent.hue,
					model: agent.model,
				}
			: { name: agentId, fullName: agentId, hue: 250, model: '' }
	}
}
