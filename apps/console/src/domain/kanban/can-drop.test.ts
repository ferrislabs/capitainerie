import { describe, expect, it } from 'vitest'
import { canDrop } from '#/domain/kanban/can-drop'
import { COLUMNS } from '#/domain/kanban/columns'

describe('canDrop', () => {
	it('allows a human to drop a card on any other column', () => {
		for (const from of COLUMNS) {
			for (const to of COLUMNS) {
				expect(canDrop(from, to)).toBe(from !== to)
			}
		}
	})
})
