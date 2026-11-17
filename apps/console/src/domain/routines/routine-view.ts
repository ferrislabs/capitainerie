import { nextRuns, scheduleText } from '#/domain/routines/schedule'
import type { PlanningTrigger, Routine } from '#/domain/routines/types'

export const planningOf = (routine: Routine): PlanningTrigger | undefined =>
	routine.triggers.find((t) => t.kind === 'planning')

export const scheduleLabel = (routine: Routine): string => {
	const planning = planningOf(routine)
	return planning ? scheduleText(planning.schedule) : 'Manuelle'
}

export function upcomingRuns(
	routine: Routine,
	now: Date,
	count: number,
): ReadonlyArray<Date> {
	const planning = planningOf(routine)
	return planning?.enabled && routine.enabled
		? nextRuns(planning.schedule, now, count)
		: []
}

/** Agents de l'équipe d'abord, puis ceux que seules les routines connaissent. */
export const agentOptions = (
	teamAgents: ReadonlyArray<string>,
	routineAgents: ReadonlyArray<string>,
): ReadonlyArray<string> => [...new Set([...teamAgents, ...routineAgents])]
