import { describe, expect, it } from 'vitest'
import {
	type Approval,
	decisionOutcome,
	describeDecideError,
	planOf,
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

describe('planOf', () => {
	const plan = (detail: Record<string, unknown>) =>
		approval('p1', { kind: 'repository_change', detail })

	it('reads the repository, the branch and the plan', () => {
		expect(
			planOf(
				plan({
					repository: 'Hypolia/lobby',
					base_branch: 'main',
					plan: '1. Retirer Pitchout',
					truncated: false,
				}),
			),
		).toEqual({
			repository: 'Hypolia/lobby',
			baseBranch: 'main',
			plan: '1. Retirer Pitchout',
			truncated: false,
		})
	})

	it('returns null for a tool call or a malformed detail', () => {
		expect(planOf(approval('t1'))).toBeNull()
		expect(planOf(plan({ repository: 'Hypolia/lobby' }))).toBeNull()
	})
})

describe('decisionOutcome for a plan', () => {
	const change = { kind: 'repository_change' as const, card_id: 'card-1' }

	it('says the agent now works on a branch', () => {
		expect(
			decisionOutcome('approve', {
				...change,
				resumed_run_id: 'run-2',
				resume_error: null,
			}).message,
		).toMatch(/pull request en brouillon/)
	})

	it('says why the agent did not start', () => {
		const outcome = decisionOutcome('approve', {
			...change,
			resumed_run_id: null,
			resume_error: 'aucun fournisseur',
		})
		expect(outcome.tone).toBe('info')
		expect(outcome.message).toBe(
			"Plan approuvé, mais l'agent n'a pas été lancé : aucun fournisseur",
		)
	})

	it('asks for a reply after a refusal', () => {
		expect(
			decisionOutcome('reject', {
				...change,
				resumed_run_id: null,
				resume_error: null,
			}).message,
		).toMatch(/réponds-y/)
	})
})
