import { describe, expect, it } from 'vitest'
import {
	COMMENT_MAX_CHARS,
	commentCanBeSent,
	commentLength,
} from '#/domain/kanban/ui/card-comments'

describe('comment length', () => {
	it('counts code points, as the server does', () => {
		expect(commentLength('a😀b')).toBe(3)
		expect(commentCanBeSent('😀'.repeat(COMMENT_MAX_CHARS))).toBe(true)
		expect(commentCanBeSent('😀'.repeat(COMMENT_MAX_CHARS + 1))).toBe(false)
	})

	it('refuses a blank comment', () => {
		expect(commentCanBeSent('  \n')).toBe(false)
	})
})
