import { describe, expect, it } from 'vitest'
import { parseTaskId, taskPathId } from '#/domain/tasks/task-ids'

describe('task ids', () => {
	it('lowercases the id for the URL', () => {
		expect(taskPathId('NAT-691')).toBe('nat-691')
	})

	it('accepts either case from the URL', () => {
		expect(parseTaskId('nat-691')).toBe('NAT-691')
		expect(parseTaskId('NAT-691')).toBe('NAT-691')
	})
})
