import { useEffect, useMemo, useState } from 'react'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { useProjects } from '#/domain/projects/hooks/use-projects'

const TICK_MS = 60_000

/** Horloge à la minute : les dates relatives (« dans 3 h ») restent justes. */
export function useNow(): number {
	const [now, setNow] = useState(() => Date.now())
	useEffect(() => {
		const timer = setInterval(() => setNow(Date.now()), TICK_MS)
		return () => clearInterval(timer)
	}, [])
	return now
}

export interface AgentOption {
	id: string
	name: string
	hue: number
}

/** Agents et projets actifs proposés dans le formulaire d'une routine. */
export function useRoutineFormOptions() {
	const agents = useTeamAgents().data?.items
	const projects = useProjects(false).data?.items
	return useMemo(
		() => ({
			agents: (agents ?? []).map(
				(a): AgentOption => ({ id: a.id, name: a.name, hue: a.hue }),
			),
			projects: (projects ?? []).map(({ project }) => ({
				id: project.id,
				name: project.name,
			})),
		}),
		[agents, projects],
	)
}
