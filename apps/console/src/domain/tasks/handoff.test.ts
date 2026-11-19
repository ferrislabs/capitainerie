import { describe, expect, it } from 'vitest'
import {
	handoffNotice,
	rememberHandoff,
	takeHandoff,
} from '#/domain/tasks/handoff'

describe('handoff', () => {
	it('says nothing when the agent started', () => {
		expect(handoffNotice({ run_id: 'r1' })).toBeNull()
		expect(handoffNotice(null)).toBeNull()
	})

	it('relays a busy agent or a missing repository as is', () => {
		expect(
			handoffNotice({ skipped: 'agent_busy', message: 'Déjà au travail' }),
		).toBe('Déjà au travail')
		expect(
			handoffNotice({ skipped: 'choose_repository', message: 'Choisis' }),
		).toBe('Choisis')
	})

	it('explains a refusal', () => {
		expect(
			handoffNotice({ skipped: 'no_provider', message: 'aucun fournisseur' }),
		).toBe("L'agent n'a pas démarré : aucun fournisseur")
	})

	it('hands the outcome over once', () => {
		rememberHandoff('c1', { run_id: 'r1' })
		expect(takeHandoff('c1')).toEqual({ run_id: 'r1' })
		expect(takeHandoff('c1')).toBeUndefined()
	})
})
