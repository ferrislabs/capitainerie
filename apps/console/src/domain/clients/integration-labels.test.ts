import { describe, expect, it } from 'vitest'
import {
	ENVIRONMENT_LABELS,
	KIND_LABELS,
	serverVersionLabel,
} from '#/domain/clients/integration-labels'

describe('integration labels', () => {
	it('names the three kinds the way the product does', () => {
		expect(KIND_LABELS).toEqual({
			odoo_json2: 'Odoo 19 et plus',
			odoo_jsonrpc: 'Odoo 18 et moins',
			n8n: 'n8n',
		})
	})

	it('names the three environments', () => {
		expect(ENVIRONMENT_LABELS).toEqual({
			production: 'Production',
			staging: 'Staging',
			development: 'Développement',
		})
	})

	it('shows the n8n major as n8n 1.x or n8n 2.x', () => {
		expect(serverVersionLabel('n8n', '1')).toBe('n8n 1.x')
		expect(serverVersionLabel('n8n', '2')).toBe('n8n 2.x')
	})

	it('keeps any other version as the server reports it', () => {
		expect(serverVersionLabel('odoo_json2', '19.0')).toBe('19.0')
		expect(serverVersionLabel('n8n', '1.76.1')).toBe('1.76.1')
	})

	it('has no label before the first check', () => {
		expect(serverVersionLabel('n8n', null)).toBeUndefined()
		expect(serverVersionLabel('odoo_json2', undefined)).toBeUndefined()
		expect(serverVersionLabel('odoo_json2', '')).toBeUndefined()
	})
})
