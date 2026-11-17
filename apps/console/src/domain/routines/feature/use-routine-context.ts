import { useEffect, useMemo, useState } from 'react'
import { useNotice } from '#/components/layout/notice'
import { DEMO_PROJECTS, useDemoState } from '#/domain/demo'
import {
	notify,
	useRoutinesState,
} from '#/domain/routines/feature/routines-store'
import { agentOptions } from '#/domain/routines/routine-view'
import { NO_PROJECT } from '#/domain/routines/types'

export function useNow(): Date {
	const [now] = useState(() => new Date())
	return now
}

export function useRoutineNotice(): string | null {
	const stored = useRoutinesState((s) => s.notice)
	const [message, show] = useNotice()
	useEffect(() => {
		if (!stored) return
		show(stored)
		notify(null)
	}, [stored, show])
	return message
}

export function useFormOptions(agent: string) {
	const team = useDemoState((s) => s.agents)
	const grants = useDemoState((s) => s.grants)
	const routines = useRoutinesState((s) => s.routines)
	return useMemo(() => {
		const known = team.find((a) => a.name === agent)
		return {
			agents: agentOptions(
				team.map((a) => a.name),
				routines.map((r) => r.agent),
			),
			projects: [NO_PROJECT, ...DEMO_PROJECTS.map((p) => p.name)],
			connectors: known ? (grants[known.id] ?? []) : [],
			agentId: known?.id,
		}
	}, [team, grants, routines, agent])
}
