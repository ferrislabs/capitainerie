import { Store } from '@tanstack/store'
import { useSyncExternalStore } from 'react'
import { DEMO_TASKS, shortName } from '#/domain/demo'
import { buildRoutines } from '#/domain/routines/mock/routines'
import {
	DEFAULT_SETTINGS,
	nameError,
	scheduleError,
	slugify,
	uniqueId,
	webhookUrl,
} from '#/domain/routines/routine-draft'
import {
	defaultSchedule,
	parseSchedule,
	type Schedule,
} from '#/domain/routines/schedule'
import type {
	Routine,
	RoutineRun,
	RoutineSettings,
	Trigger,
} from '#/domain/routines/types'

export interface NewRoutineDraft {
	settings: RoutineSettings
	schedule: Schedule
}

export interface RoutinesState {
	routines: ReadonlyArray<Routine>
	newDraft: NewRoutineDraft | null
	editDrafts: Readonly<Record<string, RoutineSettings>>
	notice: string | null
}

const initialState = (): RoutinesState => ({
	routines: buildRoutines(),
	newDraft: null,
	editDrafts: {},
	notice: null,
})

export const routinesStore = new Store<RoutinesState>(initialState())

let counter = 0
const randomSuffix = (length: number): string =>
	Math.random()
		.toString(36)
		.slice(2, 2 + length)
		.padEnd(length, '0')

export function resetRoutinesState(): void {
	counter = 0
	routinesStore.setState(() => initialState())
}

export function useRoutinesState<T>(select: (state: RoutinesState) => T): T {
	const read = () => select(routinesStore.state)
	return useSyncExternalStore(
		(onChange) => routinesStore.subscribe(onChange).unsubscribe,
		read,
		read,
	)
}

export function notify(message: string | null): void {
	routinesStore.setState((state) => ({ ...state, notice: message }))
}

function updateRoutine(
	id: string,
	change: (routine: Routine) => Routine,
	notice?: string,
): void {
	routinesStore.setState((state) => ({
		...state,
		notice: notice ?? state.notice,
		routines: state.routines.map((r) => (r.id === id ? change(r) : r)),
	}))
}

const findRoutine = (id: string): Routine | undefined =>
	routinesStore.state.routines.find((r) => r.id === id)

export function launchRoutine(id: string): void {
	const routine = findRoutine(id)
	if (!routine) return
	counter += 1
	const run: RoutineRun = {
		id: `run-${counter}`,
		when: "à l'instant",
		origin: 'Manuel',
		tone: 'neutral',
		label: 'tâche créée',
		duration: '—',
		taskId: DEMO_TASKS[0]?.id ?? 'NAT-1',
	}
	updateRoutine(
		id,
		(r) => ({
			...r,
			lastRun: { when: "À l'instant", tone: 'violet', label: 'tâche créée' },
			runs: [run, ...r.runs],
		}),
		`Routine lancée : ${routine.name}. Tâche créée pour ${shortName(routine.agent)}`,
	)
}

export function toggleRoutine(id: string): void {
	const routine = findRoutine(id)
	if (!routine) return
	updateRoutine(
		id,
		(r) => ({ ...r, enabled: !r.enabled }),
		`${routine.enabled ? 'Mise en pause' : 'Activée'} : ${routine.name}`,
	)
}

function updateTrigger(
	id: string,
	triggerId: string,
	change: (trigger: Trigger) => Trigger,
	notice?: string,
): void {
	updateRoutine(
		id,
		(r) => ({
			...r,
			triggers: r.triggers.map((t) => (t.id === triggerId ? change(t) : t)),
		}),
		notice,
	)
}

export const setTriggerEnabled = (
	id: string,
	triggerId: string,
	enabled: boolean,
): void => updateTrigger(id, triggerId, (t) => ({ ...t, enabled }))

export const patchSchedule = (
	id: string,
	triggerId: string,
	patch: Partial<Schedule>,
): void =>
	updateRoutine(id, (r) => ({
		...r,
		listSchedule: undefined,
		triggers: r.triggers.map((t) =>
			t.id === triggerId && t.kind === 'planning'
				? { ...t, schedule: { ...t.schedule, ...patch } }
				: t,
		),
	}))

export function addTrigger(id: string, kind: 'planning' | 'webhook'): void {
	const routine = findRoutine(id)
	if (!routine) return
	counter += 1
	const trigger: Trigger =
		kind === 'planning'
			? {
					kind,
					id: `trigger-${counter}`,
					enabled: true,
					schedule: parseSchedule('Chaque jour 08:00'),
				}
			: {
					kind,
					id: `trigger-${counter}`,
					enabled: true,
					url: webhookUrl(routine.name, randomSuffix(6)),
					secret: randomSuffix(4),
				}
	updateRoutine(
		id,
		(r) => ({ ...r, triggers: [...r.triggers, trigger] }),
		`${kind === 'planning' ? 'Planning' : 'Webhook'} ajouté`,
	)
}

export function removeTrigger(id: string, triggerId: string): void {
	updateRoutine(id, (r) => ({
		...r,
		triggers: r.triggers.filter((t) => t.id !== triggerId),
	}))
}

export const regenerateSecret = (id: string, triggerId: string): void =>
	updateTrigger(
		id,
		triggerId,
		(t) => (t.kind === 'webhook' ? { ...t, secret: randomSuffix(4) } : t),
		"Nouveau secret généré. Mets à jour l'appelant.",
	)

export function setEditDraft(id: string, settings: RoutineSettings): void {
	routinesStore.setState((state) => ({
		...state,
		editDrafts: { ...state.editDrafts, [id]: settings },
	}))
}

export function discardEditDraft(id: string): void {
	routinesStore.setState((state) => {
		const { [id]: _dropped, ...rest } = state.editDrafts
		return { ...state, editDrafts: rest, notice: 'Modifications annulées' }
	})
}

/** Enregistre le brouillon ; l'identifiant (donc l'URL) ne suit pas le nom. */
export function saveEditDraft(id: string): boolean {
	const draft = routinesStore.state.editDrafts[id]
	if (!draft) return false
	const error = nameError(draft.name)
	if (error) {
		notify(error)
		return false
	}
	routinesStore.setState((state) => {
		const { [id]: _dropped, ...rest } = state.editDrafts
		return {
			...state,
			editDrafts: rest,
			notice: 'Routine enregistrée',
			routines: state.routines.map((r) =>
				r.id === id ? { ...r, ...draft, name: draft.name.trim() } : r,
			),
		}
	})
	return true
}

export const blankDraft = (): NewRoutineDraft => ({
	settings: DEFAULT_SETTINGS,
	schedule: defaultSchedule(),
})

export function setNewDraft(draft: NewRoutineDraft): void {
	routinesStore.setState((state) => ({ ...state, newDraft: draft }))
}

/** Crée la routine du brouillon ; renvoie son identifiant, ou `null` si une règle échoue. */
export function createRoutine(): string | null {
	const draft = routinesStore.state.newDraft ?? blankDraft()
	const error =
		(nameError(draft.settings.name) && 'Donne un nom à la routine') ||
		scheduleError(draft.schedule)
	if (error) {
		notify(error)
		return null
	}
	const name = draft.settings.name.trim()
	const id = uniqueId(
		slugify(name),
		routinesStore.state.routines.map((r) => r.id),
	)
	counter += 1
	const routine: Routine = {
		...draft.settings,
		id,
		name,
		enabled: true,
		lastRun: { when: 'Jamais', tone: 'neutral', label: 'jamais lancée' },
		triggers: [
			{
				kind: 'planning',
				id: `trigger-${counter}`,
				enabled: true,
				schedule: draft.schedule,
			},
		],
		runs: [],
	}
	routinesStore.setState((state) => ({
		...state,
		newDraft: null,
		notice: `Routine créée : ${name}`,
		routines: [...state.routines, routine],
	}))
	return id
}
