import { describe, expect, it } from 'vitest'
import { filterInbox, type InboxItem } from '#/domain/inbox/filter-inbox'

const item = (
	id: string,
	scope: InboxItem['scope'],
	unread: boolean,
): InboxItem => ({
	id,
	kind: 'task',
	ref: id,
	title: id,
	source: 's',
	when: '1 h',
	unread,
	scope,
})

const ITEMS = [
	item('a', 'moi', false),
	item('b', 'moi', true),
	item('c', 'recent', true),
]

describe('filterInbox', () => {
	it('keeps everything on tous', () => {
		expect(filterInbox('tous', ITEMS).map((i) => i.id)).toEqual(['a', 'b', 'c'])
	})

	it('filters by scope', () => {
		expect(filterInbox('moi', ITEMS).map((i) => i.id)).toEqual(['a', 'b'])
		expect(filterInbox('recent', ITEMS).map((i) => i.id)).toEqual(['c'])
	})

	it('keeps unread items on non-lus', () => {
		expect(filterInbox('non-lus', ITEMS).map((i) => i.id)).toEqual(['b', 'c'])
	})

	it('returns an empty list when nothing matches', () => {
		expect(filterInbox('moi', [])).toEqual([])
	})
})
