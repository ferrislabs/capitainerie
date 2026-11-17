import { describe, expect, it } from 'vitest'
import {
	hueOf,
	initialOf,
	slugify,
	uniqueAgentId,
} from '#/domain/agent-team/agent-identity'

describe('agent identity', () => {
	it('derives a stable hue under 360', () => {
		expect(hueOf('Darnold')).toBe(hueOf('Darnold'))
		expect(hueOf('Darnold')).toBeLessThan(360)
		expect(hueOf('Darnold')).not.toBe(hueOf('Zoé'))
	})

	it('takes the upper-case initial', () => {
		expect(initialOf('  darnold')).toBe('D')
		expect(initialOf('')).toBe('?')
	})

	it('slugifies without accents', () => {
		expect(slugify('Zoé (Relais)')).toBe('zoe-relais')
	})

	it('keeps ids unique', () => {
		expect(uniqueAgentId('Dev', ['dev'])).toBe('dev-2')
		expect(uniqueAgentId('Dev', ['dev', 'dev-2'])).toBe('dev-3')
		expect(uniqueAgentId('Nouveau', ['dev'])).toBe('nouveau')
	})
})
