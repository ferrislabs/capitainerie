import { describe, expect, it } from 'vitest'
import {
	type Approval,
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
