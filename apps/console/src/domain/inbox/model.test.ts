import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { targetOf } from '#/domain/inbox/model'

const item = (
	extra: Partial<Schemas.InboxItemResponse> = {},
): Schemas.InboxItemResponse => ({
	id: 'i1',
	kind: 'approval',
	title: 't',
	body: 'b',
	created_at: '2026-11-18T10:00:00Z',
	unread: true,
	...extra,
})

describe('targetOf', () => {
	it('prefers the linked task', () => {
		expect(targetOf(item({ card_id: 'c1', link: '/approvals' }))).toEqual({
			kind: 'task',
			taskId: 'c1',
		})
	})

	it('follows an internal or external link', () => {
		expect(targetOf(item({ link: '/routines/r1' }))).toEqual({
			kind: 'internal',
			href: '/routines/r1',
		})
		expect(targetOf(item({ link: 'https://example.com/x' }))).toEqual({
			kind: 'external',
			href: 'https://example.com/x',
		})
	})

	it('ignores unsafe links and falls back on the page of the kind', () => {
		expect(targetOf(item({ link: 'javascript:alert(1)' }))).toEqual({
			kind: 'page',
			to: '/approvals',
		})
		expect(targetOf(item({ link: '//evil.test' }))).toEqual({
			kind: 'page',
			to: '/approvals',
		})
		expect(targetOf(item({ kind: 'run' }))).toEqual({
			kind: 'page',
			to: '/audit/{-$tab}',
		})
		expect(targetOf(item({ kind: 'source' }))).toEqual({
			kind: 'page',
			to: '/sources',
		})
		expect(targetOf(item({ kind: 'routine' }))).toEqual({
			kind: 'page',
			to: '/routines',
		})
		expect(targetOf(item({ kind: 'card' }))).toEqual({
			kind: 'page',
			to: '/tasks',
		})
	})
})
