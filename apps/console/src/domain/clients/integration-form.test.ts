import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	EMPTY_VALUES,
	type IntegrationValues,
	integrationBody,
	integrationPatch,
	validateIntegration,
	valuesOf,
} from '#/domain/clients/integration-form'

const filled: IntegrationValues = {
	kind: 'odoo_json2',
	name: ' Odoo prod ',
	environment: 'production',
	baseUrl: ' https://acme.odoo.com ',
	database: '',
	login: '',
	secret: 'key-123',
}

const STORED: Schemas.IntegrationInstanceResponse = {
	id: 'i1',
	client_id: 'cl1',
	kind: 'odoo_jsonrpc',
	name: 'Odoo prod',
	environment: 'production',
	base_url: 'https://acme.odoo.com',
	database: 'acme',
	login: 'api@acme.test',
	secret_preview: '…c3d4',
	server_version: '17.0',
	checked_at: '2026-10-20T08:00:00Z',
	created_by: 'ada',
	created_at: '2026-10-16T09:00:00Z',
	updated_at: '2026-10-16T09:00:00Z',
	archived_at: null,
}

describe('validateIntegration', () => {
	it('asks for a name, an address and a key when adding', () => {
		expect(validateIntegration(EMPTY_VALUES, 'create')).toEqual({
			name: 'Le nom est obligatoire.',
			baseUrl: 'L’adresse est obligatoire.',
			secret: 'La clé est obligatoire.',
		})
	})

	it('needs the base and the login for Odoo 18 and earlier only', () => {
		const jsonrpc = { ...filled, kind: 'odoo_jsonrpc' as const }
		expect(validateIntegration(jsonrpc, 'create')).toEqual({
			database: 'La base est obligatoire.',
			login: 'L’identifiant est obligatoire.',
		})
		expect(validateIntegration(filled, 'create')).toEqual({})
		expect(
			validateIntegration({ ...filled, kind: 'n8n' as const }, 'create'),
		).toEqual({})
	})

	it('accepts a blank key when editing, to keep the stored one', () => {
		expect(validateIntegration({ ...filled, secret: '' }, 'edit')).toEqual({})
		expect(validateIntegration({ ...filled, secret: '   ' }, 'create')).toEqual(
			{ secret: 'La clé est obligatoire.' },
		)
	})
})

describe('integrationBody', () => {
	it('leaves out the base when empty and never sends a login to Odoo 19', () => {
		expect(integrationBody({ ...filled, login: 'x' })).toEqual({
			kind: 'odoo_json2',
			name: 'Odoo prod',
			environment: 'production',
			base_url: 'https://acme.odoo.com',
			secret: 'key-123',
		})
		expect(integrationBody({ ...filled, database: ' acme ' })).toMatchObject({
			database: 'acme',
		})
	})

	it('sends the base and the login for Odoo 18 and earlier', () => {
		expect(
			integrationBody({
				...filled,
				kind: 'odoo_jsonrpc',
				database: ' acme ',
				login: ' api@acme.test ',
			}),
		).toMatchObject({ database: 'acme', login: 'api@acme.test' })
	})

	it('sends neither for n8n and keeps the key as typed', () => {
		const body = integrationBody({
			...filled,
			kind: 'n8n',
			database: 'acme',
			login: 'x',
			secret: ' k ',
		})
		expect(body).not.toHaveProperty('database')
		expect(body).not.toHaveProperty('login')
		expect(body.secret).toBe(' k ')
	})
})

describe('integrationPatch', () => {
	const unchanged = { ...valuesOf(STORED), secret: '' }

	it('starts from the stored values with an empty key', () => {
		expect(unchanged).toEqual({
			kind: 'odoo_jsonrpc',
			name: 'Odoo prod',
			environment: 'production',
			baseUrl: 'https://acme.odoo.com',
			database: 'acme',
			login: 'api@acme.test',
			secret: '',
		})
		expect(integrationPatch(unchanged, STORED)).toEqual({})
	})

	it('sends only what changed', () => {
		expect(
			integrationPatch(
				{ ...unchanged, name: ' Odoo recette ', environment: 'staging' },
				STORED,
			),
		).toEqual({ name: 'Odoo recette', environment: 'staging' })
	})

	it('sends a new key, and only when one was typed', () => {
		expect(integrationPatch({ ...unchanged, secret: 'new' }, STORED)).toEqual({
			secret: 'new',
		})
	})

	it('sends a changed address, base or login', () => {
		expect(
			integrationPatch(
				{
					...unchanged,
					baseUrl: 'https://other.odoo.com',
					database: 'other',
					login: 'bot',
				},
				STORED,
			),
		).toEqual({
			base_url: 'https://other.odoo.com',
			database: 'other',
			login: 'bot',
		})
	})

	it('clears the base of Odoo 19 with an empty string', () => {
		const json2 = { ...STORED, kind: 'odoo_json2' as const, login: null }
		expect(
			integrationPatch({ ...valuesOf(json2), secret: '', database: '' }, json2),
		).toEqual({ database: '' })
	})

	it('never sends a base or a login for n8n', () => {
		const n8n = { ...STORED, kind: 'n8n' as const, database: null, login: null }
		expect(
			integrationPatch(
				{ ...valuesOf(n8n), secret: '', database: 'x', login: 'y' },
				n8n,
			),
		).toEqual({})
	})
})
