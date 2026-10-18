import { describe, expect, it } from 'vitest'
import { parseExecuteOutput } from '#/domain/runs/execute-output'

describe('parseExecuteOutput', () => {
	it('reads the keys written by the execute pipeline', () => {
		expect(
			parseExecuteOutput({
				branch: 'capitainerie/0123456789ab',
				pull_request: 'https://github.com/acme/web/pull/7',
				no_changes: false,
				dropped_tool_calls: 3,
				error: null,
			}),
		).toEqual({
			branch: 'capitainerie/0123456789ab',
			pullRequest: 'https://github.com/acme/web/pull/7',
			noChanges: false,
			droppedToolCalls: 3,
			error: undefined,
		})
	})

	it('ignores a pull request that is not an http link', () => {
		expect(
			parseExecuteOutput({ pull_request: 'javascript:alert(1)' })?.pullRequest,
		).toBeUndefined()
	})

	it('returns nothing without an object', () => {
		expect(parseExecuteOutput(undefined)).toBeUndefined()
		expect(parseExecuteOutput(null)).toBeUndefined()
		expect(parseExecuteOutput('x')).toBeUndefined()
	})
})
