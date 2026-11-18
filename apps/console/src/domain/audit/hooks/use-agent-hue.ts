import { useCallback } from 'react'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { hueOf } from '#/domain/dashboard/format'

/** Teinte d'avatar d'un agent : celle de l'équipe si connue, sinon dérivée du nom. */
export function useAgentHue(): (
	id: string | null | undefined,
	name: string,
) => number {
	const agents = useTeamAgents().data?.items
	return useCallback(
		(id, name) =>
			agents?.find((agent) => agent.id === id || agent.name === name)?.hue ??
			hueOf(name),
		[agents],
	)
}
