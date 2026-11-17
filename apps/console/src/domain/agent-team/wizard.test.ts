import { describe, expect, it } from 'vitest'
import {
	buildNewAgent,
	canAdvance,
	initialWizard,
	toggled,
} from '#/domain/agent-team/wizard'

describe('wizard', () => {
	it('blocks until name, adapter and key are set', () => {
		const w = initialWizard()
		expect(canAdvance({ ...w, step: 0 })).toBe(false)
		expect(canAdvance({ ...w, step: 0, name: ' Zoé ' })).toBe(true)
		expect(canAdvance({ ...w, step: 1 })).toBe(false)
		expect(canAdvance({ ...w, step: 1, adapter: 'Pi' })).toBe(true)
		expect(canAdvance({ ...w, step: 2, conn: 'key' })).toBe(false)
		expect(canAdvance({ ...w, step: 2, conn: 'key', key: 'sk' })).toBe(true)
		expect(canAdvance({ ...w, step: 2 })).toBe(true)
	})

	it('starts with no connector and no source', () => {
		const w = initialWizard()
		expect(w.connectors).toEqual([])
		expect(w.sources).toEqual([])
	})

	it('toggles list entries', () => {
		expect(toggled(['a'], 'b')).toEqual(['a', 'b'])
		expect(toggled(['a', 'b'], 'a')).toEqual(['b'])
	})

	it('builds the agent', () => {
		const w = {
			...initialWizard(),
			name: ' Zoé ',
			reports: 'Aucun (direction)',
		}
		const agent = buildNewAgent(w, 'zoe')
		expect(agent).toMatchObject({
			id: 'zoe',
			name: 'Zoé',
			reportsTo: null,
			status: 'idle',
		})
	})
})
