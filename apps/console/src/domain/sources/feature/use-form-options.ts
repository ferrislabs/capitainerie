import { useMemo } from 'react'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { useProjects } from '#/domain/projects/hooks/use-projects'

/** Agents et projets actifs proposés dans le formulaire d'une source. */
export function useSourceFormOptions() {
	const agents = useTeamAgents().data?.items
	const projects = useProjects(false).data?.items
	return useMemo(
		() => ({
			agents: (agents ?? []).map((a) => ({ id: a.id, name: a.name })),
			projects: (projects ?? []).map(({ project }) => ({
				id: project.id,
				name: project.name,
			})),
		}),
		[agents, projects],
	)
}
