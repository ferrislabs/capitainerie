import { beforeEach, describe, expect, it } from 'vitest'
import {
	addTrigger,
	blankDraft,
	createRoutine,
	launchRoutine,
	patchSchedule,
	regenerateSecret,
	removeTrigger,
	resetRoutinesState,
	routinesStore,
	saveEditDraft,
	setEditDraft,
	setNewDraft,
	toggleRoutine,
} from '#/domain/routines/feature/routines-store'

const ID = 'analyse-board-hebdomadaire'
const get = (id: string) =>
	routinesStore.state.routines.find((r) => r.id === id)

beforeEach(() => resetRoutinesState())

describe('fixtures', () => {
	it('has eleven routines with accent-free slugs', () => {
		expect(routinesStore.state.routines).toHaveLength(11)
		expect(get('marketing-declencheurs-de-publication')).toBeDefined()
	})
})

describe('launchRoutine', () => {
	it('adds a run and updates the last run', () => {
		const before = get(ID)?.runs.length ?? 0
		launchRoutine(ID)
		expect(get(ID)?.runs).toHaveLength(before + 1)
		expect(get(ID)?.lastRun.when).toBe("À l'instant")
		expect(routinesStore.state.notice).toContain('Routine lancée')
	})
})

describe('toggleRoutine', () => {
	it('flips the enabled flag', () => {
		toggleRoutine(ID)
		expect(get(ID)?.enabled).toBe(false)
	})
})

describe('triggers', () => {
	it('adds, edits and removes triggers', () => {
		addTrigger(ID, 'webhook')
		const hook = get(ID)?.triggers.find((t) => t.kind === 'webhook')
		expect(hook?.kind === 'webhook' && hook.url).toContain('/hooks/routines/')
		if (!hook || hook.kind !== 'webhook') throw new Error('no hook')
		regenerateSecret(ID, hook.id)
		removeTrigger(ID, hook.id)
		expect(get(ID)?.triggers).toHaveLength(1)
		const planning = get(ID)?.triggers[0]
		if (!planning) throw new Error('no planning')
		patchSchedule(ID, planning.id, { time: '07:15' })
		const patched = get(ID)?.triggers[0]
		expect(patched?.kind === 'planning' && patched.schedule.time).toBe('07:15')
	})
})

describe('saveEditDraft', () => {
	it('refuses an empty name and keeps the draft', () => {
		const routine = get(ID)
		if (!routine) throw new Error('missing')
		setEditDraft(ID, { ...routine, name: ' ' })
		expect(saveEditDraft(ID)).toBe(false)
		expect(get(ID)?.name).toBe(routine.name)
	})
	it('applies the draft and keeps the id', () => {
		const routine = get(ID)
		if (!routine) throw new Error('missing')
		setEditDraft(ID, { ...routine, name: 'Nouveau nom', priority: 'Haute' })
		expect(saveEditDraft(ID)).toBe(true)
		expect(get(ID)?.name).toBe('Nouveau nom')
		expect(get(ID)?.priority).toBe('Haute')
	})
})

describe('createRoutine', () => {
	it('rejects a missing name', () => {
		expect(createRoutine()).toBeNull()
		expect(routinesStore.state.routines).toHaveLength(11)
	})
	it('rejects a weekly schedule without day', () => {
		const draft = blankDraft()
		setNewDraft({
			settings: { ...draft.settings, name: 'Test' },
			schedule: { ...draft.schedule, days: [] },
		})
		expect(createRoutine()).toBeNull()
		expect(routinesStore.state.notice).toBe('Choisis au moins un jour')
	})
	it('creates the routine with its schedule', () => {
		const draft = blankDraft()
		setNewDraft({
			...draft,
			settings: { ...draft.settings, name: 'Été : bilan' },
		})
		expect(createRoutine()).toBe('ete-bilan')
		expect(get('ete-bilan')?.runs).toEqual([])
		expect(routinesStore.state.newDraft).toBeNull()
	})
})
