import { describe, expect, it } from 'vitest'
import { interveneAvailability } from '#/domain/integrations/intervene/availability'
import { activeInstances } from '#/domain/integrations/intervene/grants'
import {
	N8N_DEV,
	ODOO_OLD,
	ODOO_PROD,
} from '#/domain/integrations/intervene/test-fixtures'

const CLIENT = { id: 'cl1', name: 'Acme' }

describe('activeInstances', () => {
	it('drops the archived ones', () => {
		expect(activeInstances([ODOO_PROD, ODOO_OLD, N8N_DEV])).toEqual([
			ODOO_PROD,
			N8N_DEV,
		])
	})
})

describe('interveneAvailability', () => {
	it('asks to link the card to a client first', () => {
		const expected = {
			kind: 'blocked',
			reason: 'Rattachez d’abord la carte à un client',
		}
		expect(interveneAvailability(undefined, undefined)).toEqual(expected)
		expect(interveneAvailability(null, [ODOO_PROD])).toEqual(expected)
	})

	it('says when the client has no active integration', () => {
		const expected = {
			kind: 'blocked',
			reason: 'Ce client n’a aucune intégration',
		}
		expect(interveneAvailability(CLIENT, [])).toEqual(expected)
		expect(interveneAvailability(CLIENT, [ODOO_OLD])).toEqual(expected)
	})

	it('waits for the instances to load', () => {
		expect(interveneAvailability(CLIENT, undefined)).toEqual({
			kind: 'loading',
		})
	})

	it('is ready with one active instance', () => {
		expect(interveneAvailability(CLIENT, [ODOO_OLD, N8N_DEV])).toEqual({
			kind: 'ready',
		})
	})
})
