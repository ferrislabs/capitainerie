import { describe, expect, it } from 'vitest'
import {
	type Approval,
	decisionOutcome,
	describeDecideError,
	prettyJson,
	toolCallOf,
} from '#/domain/approvals/model'
import { approval } from '#/domain/approvals/test-fixtures'

describe('toolCallOf', () => {
	it('reads the tool and its arguments', () => {
		expect(
			toolCallOf(
				approval('a1', {
					detail: { tool: 'gmail.send', arguments: { to: 'x@y.fr' } },
				}),
			),
		).toEqual({ tool: 'gmail.send', arguments: { to: 'x@y.fr' } })
	})

	it('returns null for other kinds or a malformed detail', () => {
		const action: Approval = approval('a2', {
			kind: 'agent_action',
			detail: { tool: 'x' },
		})
		expect(toolCallOf(action)).toBeNull()
		expect(toolCallOf(approval('a3', { detail: {} }))).toBeNull()
	})
})

it('pretty prints JSON', () => {
	expect(prettyJson({ a: 1 })).toBe('{\n  "a": 1\n}')
})

it('explains a 409', () => {
	expect(describeDecideError({ status: 409 })).toMatch(/plus en attente/)
	expect(describeDecideError(null)).toMatch(/Réessaie/)
})

describe('decisionOutcome', () => {
	const call = { kind: 'tool_call' as const, card_id: 'card-1' }

	it('says the run was resumed and links the task', () => {
		expect(
			decisionOutcome('approve', {
				...call,
				resumed_run_id: 'run-2',
				resume_error: null,
			}),
		).toEqual({
			tone: 'success',
			message: 'Exécution relancée.',
			cardId: 'card-1',
		})
	})

	it('carries why the run was not resumed', () => {
		const outcome = decisionOutcome('approve', {
			...call,
			resumed_run_id: null,
			resume_error: "l'agent assigné est en pause",
		})
		expect(outcome.tone).toBe('info')
		expect(outcome.message).toMatch(/n'a pas été relancée : l'agent assigné/)
		expect(outcome.cardId).toBe('card-1')
	})

	it('confirms a rejection, noted on the task for a tool call', () => {
		expect(
			decisionOutcome('reject', {
				...call,
				resumed_run_id: null,
				resume_error: null,
			}).message,
		).toMatch(/noté sur la tâche/)
		expect(
			decisionOutcome('reject', {
				kind: 'agent_action',
				card_id: null,
				resumed_run_id: null,
				resume_error: null,
			}),
		).toEqual({ tone: 'success', message: 'Demande refusée.', cardId: null })
	})
})
