import { describe, expect, it } from 'vitest'
import {
	canSubmitIntervention,
	INSTRUCTIONS_MAX_CHARS,
	instructionsLength,
	interveneBody,
	needsProductionConfirmation,
} from '#/domain/integrations/intervene/grants'
import {
	N8N_DEV,
	ODOO_PROD,
	ODOO_STAGING,
} from '#/domain/integrations/intervene/test-fixtures'

const INSTANCES = [ODOO_PROD, ODOO_STAGING, N8N_DEV]

function form(
	choices: Record<string, 'none' | 'read' | 'write'>,
	extra: { instructions?: string; confirmed?: boolean } = {},
) {
	return { choices, instructions: '', confirmed: false, ...extra }
}

describe('needsProductionConfirmation', () => {
	it('is only true for a production instance in write', () => {
		expect(needsProductionConfirmation(INSTANCES, { i1: 'write' })).toBe(true)
		expect(needsProductionConfirmation(INSTANCES, { i1: 'read' })).toBe(false)
		expect(needsProductionConfirmation(INSTANCES, { i1: 'none' })).toBe(false)
		expect(
			needsProductionConfirmation(INSTANCES, { i2: 'write', i3: 'write' }),
		).toBe(false)
		expect(needsProductionConfirmation(INSTANCES, {})).toBe(false)
	})
})

describe('canSubmitIntervention', () => {
	it('needs at least one instance above « Aucun »', () => {
		expect(canSubmitIntervention(INSTANCES, form({}))).toBe(false)
		expect(canSubmitIntervention(INSTANCES, form({ i2: 'none' }))).toBe(false)
		expect(canSubmitIntervention(INSTANCES, form({ i2: 'read' }))).toBe(true)
		expect(canSubmitIntervention(INSTANCES, form({ i3: 'write' }))).toBe(true)
	})

	it('ignores the choice made for an instance that is gone', () => {
		expect(canSubmitIntervention(INSTANCES, form({ gone: 'write' }))).toBe(
			false,
		)
	})

	it('waits for the production confirmation', () => {
		expect(canSubmitIntervention(INSTANCES, form({ i1: 'write' }))).toBe(false)
		expect(
			canSubmitIntervention(
				INSTANCES,
				form({ i1: 'write' }, { confirmed: true }),
			),
		).toBe(true)
		expect(canSubmitIntervention(INSTANCES, form({ i1: 'read' }))).toBe(true)
	})

	it('refuses instructions past the limit', () => {
		const long = 'é'.repeat(INSTRUCTIONS_MAX_CHARS + 1)
		expect(
			canSubmitIntervention(
				INSTANCES,
				form(
					{ i2: 'read' },
					{ instructions: 'é'.repeat(INSTRUCTIONS_MAX_CHARS) },
				),
			),
		).toBe(true)
		expect(
			canSubmitIntervention(
				INSTANCES,
				form({ i2: 'read' }, { instructions: long }),
			),
		).toBe(false)
	})
})

describe('instructionsLength', () => {
	it('counts characters, not UTF-16 units', () => {
		expect(instructionsLength('abc')).toBe(3)
		expect(instructionsLength('a😀')).toBe(2)
	})
})

describe('interveneBody', () => {
	it('sends the grants above « Aucun » with the card version', () => {
		expect(
			interveneBody(
				INSTANCES,
				form({ i1: 'none', i2: 'read', i3: 'write' }),
				5,
			),
		).toEqual({
			grants: [
				{ instance_id: 'i2', access: 'read' },
				{ instance_id: 'i3', access: 'write' },
			],
			expected_version: 5,
		})
	})

	it('trims the instructions and leaves them out when blank', () => {
		expect(
			interveneBody(
				INSTANCES,
				form(
					{ i2: 'read' },
					{ instructions: '  Vérifier dans Odoo staging \n' },
				),
				1,
			).instructions,
		).toBe('Vérifier dans Odoo staging')
		expect(
			interveneBody(
				INSTANCES,
				form({ i2: 'read' }, { instructions: '  \n ' }),
				1,
			),
		).not.toHaveProperty('instructions')
	})

	it('confirms production writes only when one is asked for and ticked', () => {
		expect(
			interveneBody(INSTANCES, form({ i1: 'write' }, { confirmed: true }), 1)
				.confirm_production_writes,
		).toBe(true)
		expect(
			interveneBody(INSTANCES, form({ i1: 'write' }), 1),
		).not.toHaveProperty('confirm_production_writes')
		expect(
			interveneBody(INSTANCES, form({ i1: 'read' }, { confirmed: true }), 1),
		).not.toHaveProperty('confirm_production_writes')
	})
})
