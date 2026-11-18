import { describe, expect, it } from 'vitest'
import {
	buildCreateRequest,
	buildWizardPatch,
	canAdvance,
	initialWizard,
	toggled,
} from '#/domain/agent-team/wizard'
import { DEV } from '#/test-utils/team-agents'

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
		expect(canAdvance({ ...w, step: 3 })).toBe(true)
		expect(canAdvance({ ...w, step: 3, model: 'Claude Sonnet 5.5' })).toBe(
			false,
		)
		expect(canAdvance({ ...w, step: 3, model: 'claude-sonnet-5-5' })).toBe(true)
	})

	it('starts with no connector and no parent', () => {
		const w = initialWizard()
		expect(w.connectors).toEqual([])
		expect(w.reports).toBe('Aucun (direction)')
	})

	it('toggles list entries', () => {
		expect(toggled(['a'], 'b')).toEqual(['a', 'b'])
		expect(toggled(['a', 'b'], 'a')).toEqual(['b'])
	})

	it('builds the create request', () => {
		const w = {
			...initialWizard(),
			name: ' Zoé ',
			title: ' Relais ',
			instr: ' Fais ça ',
			reports: 'a-chief',
			connectors: ['Chatwoot', 'Slack'],
		}
		const body = buildCreateRequest(w, 'zoe', ['Chatwoot'])
		expect(body).toMatchObject({
			slug: 'zoe',
			name: 'Zoé',
			description: 'Relais',
			instructions: 'Fais ça',
			reports_to: 'a-chief',
			connector_grants: ['Chatwoot'],
			role: 'Général',
			model: '',
		})
		expect(typeof body.hue).toBe('number')
	})

	it('omits instructions when blank and maps the direction to null', () => {
		const body = buildCreateRequest(
			{ ...initialWizard(), name: 'Zoé' },
			'zoe',
			[],
		)
		expect(body).not.toHaveProperty('instructions')
		expect(body.reports_to).toBeNull()
	})

	it('builds the configuration patch sent after creation', () => {
		const patch = buildWizardPatch(
			{
				...initialWizard(),
				adapter: 'Codex',
				interval: 'Manuelle',
				skills: ['humanizer'],
			},
			DEV,
		)
		expect(patch.config).toMatchObject({
			adapter: 'Codex',
			hb: false,
			title: 'Général',
		})
		expect(patch.skills).toEqual(['humanizer'])
		expect(patch.change_summary).toBe('configuration initiale')
	})
})
