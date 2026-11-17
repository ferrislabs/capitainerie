import { useEffect, useMemo, useState } from 'react'
import { DEMO_PROJECTS, useDemoState } from '#/domain/demo'
import {
	notify,
	useRoutinesState,
} from '#/domain/routines/feature/routines-store'
import { agentOptions } from '#/domain/routines/routine-view'
import { NO_PROJECT } from '#/domain/routines/types'

const NOTICE_MS = 5000

export function useNow(): Date {
	const [now] = useState(() => new Date())
	return now
}

export function useNotice(): string | null {
	const notice = useRoutinesState((s) => s.notice)
	useEffect(() => {
		if (!notice) return
		const timer = window.setTimeout(() => notify(null), NOTICE_MS)
		return () => window.clearTimeout(timer)
	}, [notice])
	return notice
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
		}
	}, [team, grants, routines, agent])
}
