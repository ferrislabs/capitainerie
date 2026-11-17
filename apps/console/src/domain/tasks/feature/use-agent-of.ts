import { shortName, useDemoState } from '#/domain/demo'
import type { AgentView } from '#/domain/tasks/ui/tasks-view'

export function useAgentOf(): (agentId: string) => AgentView {
	const agents = useDemoState((s) => s.agents)
	return (agentId) => {
		const agent = agents.find((a) => a.id === agentId)
		return agent
			? { name: shortName(agent.name), hue: agent.hue }
			: { name: agentId, hue: 0 }
	}
}
