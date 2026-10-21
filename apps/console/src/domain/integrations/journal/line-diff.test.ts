import { describe, expect, it } from 'vitest'
import { hasChanges, lineDiff } from '#/domain/integrations/journal/line-diff'

describe('lineDiff', () => {
	it('keeps identical texts as unchanged lines', () => {
		expect(lineDiff('a\nb', 'a\nb')).toEqual([
			{ kind: 'same', text: 'a' },
			{ kind: 'same', text: 'b' },
		])
	})

	it('marks a changed line as removed then added', () => {
		expect(lineDiff('a\nb\nc', 'a\nB\nc')).toEqual([
			{ kind: 'same', text: 'a' },
			{ kind: 'removed', text: 'b' },
			{ kind: 'added', text: 'B' },
			{ kind: 'same', text: 'c' },
		])
	})

	it('marks inserted and deleted lines', () => {
		expect(lineDiff('a\nc', 'a\nb\nc')).toEqual([
			{ kind: 'same', text: 'a' },
			{ kind: 'added', text: 'b' },
			{ kind: 'same', text: 'c' },
		])
		expect(lineDiff('a\nb\nc', 'a\nc')).toEqual([
			{ kind: 'same', text: 'a' },
			{ kind: 'removed', text: 'b' },
			{ kind: 'same', text: 'c' },
		])
	})

	it('finds the longest common run across several edits', () => {
		const before = ['{', '  "a": 1,', '  "b": 2,', '  "c": 3', '}'].join('\n')
		const after = [
			'{',
			'  "a": 1,',
			'  "b": 20,',
			'  "c": 3,',
			'  "d": 4',
			'}',
		].join('\n')
		expect(lineDiff(before, after)).toEqual([
			{ kind: 'same', text: '{' },
			{ kind: 'same', text: '  "a": 1,' },
			{ kind: 'removed', text: '  "b": 2,' },
			{ kind: 'removed', text: '  "c": 3' },
			{ kind: 'added', text: '  "b": 20,' },
			{ kind: 'added', text: '  "c": 3,' },
			{ kind: 'added', text: '  "d": 4' },
			{ kind: 'same', text: '}' },
		])
	})

	it('shows everything as added from an empty text and as removed to one', () => {
		expect(lineDiff('', 'a\nb')).toEqual([
			{ kind: 'added', text: 'a' },
			{ kind: 'added', text: 'b' },
		])
		expect(lineDiff('a\nb', '')).toEqual([
			{ kind: 'removed', text: 'a' },
			{ kind: 'removed', text: 'b' },
		])
		expect(lineDiff('', '')).toEqual([])
	})

	it('falls back to a coarse replacement on very large inputs', () => {
		const before = Array.from({ length: 3000 }, (_, i) => `a${i}`).join('\n')
		const after = Array.from({ length: 3000 }, (_, i) => `b${i}`).join('\n')
		const diff = lineDiff(before, after)
		expect(diff).toHaveLength(6000)
		expect(diff.slice(0, 3000).every((line) => line.kind === 'removed')).toBe(
			true,
		)
		expect(diff.slice(3000).every((line) => line.kind === 'added')).toBe(true)
	})
})

describe('hasChanges', () => {
	it('tells whether any line differs', () => {
		expect(hasChanges(lineDiff('a', 'a'))).toBe(false)
		expect(hasChanges(lineDiff('a', 'b'))).toBe(true)
		expect(hasChanges([])).toBe(false)
	})
})
