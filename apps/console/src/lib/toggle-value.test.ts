import { describe, expect, it } from 'vitest'
import { toggleValue } from '#/lib/toggle-value'

describe('toggleValue', () => {
	it('adds then removes a value', () => {
		expect(toggleValue([], 'a')).toEqual(['a'])
		expect(toggleValue(['a', 'b'], 'a')).toEqual(['b'])
	})
})
