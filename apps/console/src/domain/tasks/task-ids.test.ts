import { describe, expect, it } from 'vitest'
import { shortTaskId, taskPathId } from '#/domain/tasks/task-ids'

describe('task ids', () => {
	it('lowercases the id for the URL', () => {
		expect(taskPathId('NAT-691')).toBe('nat-691')
	})

	it('keeps the first 8 characters of a uuid', () => {
		expect(shortTaskId('0198a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b')).toBe('0198a1b2')
	})
})
