import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	EMPTY_VALUES,
	type IntegrationValues,
	integrationBody,
	integrationPatch,
	requiresNewSecret,
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
		expect(validateIntegration(valuesOf(STORED), 'edit', STORED)).toEqual({})
	})

	it('asks for a new key when editing changes the origin, the base or the login', () => {
		const unchanged = valuesOf(STORED)
		const required = { secret: 'La clé est obligatoire.' }
		for (const changed of [
			{ baseUrl: 'https://other.odoo.com' },
			{ database: 'other' },
			{ login: 'bot' },
		]) {
			expect(
				validateIntegration({ ...unchanged, ...changed }, 'edit', STORED),
			).toEqual(required)
			expect(
				validateIntegration(
					{ ...unchanged, ...changed, secret: 'new' },
					'edit',
					STORED,
				),
			).toEqual({})
		}
	})

	it('keeps the key optional when editing changes only the path', () => {
		expect(
			validateIntegration(
				{ ...valuesOf(STORED), baseUrl: 'https://acme.odoo.com/odoo' },
				'edit',
				STORED,
			),
		).toEqual({})
	})

	it('keeps a whitespace key from satisfying the new key requirement', () => {
		expect(
			validateIntegration(
				{ ...valuesOf(STORED), database: 'other', secret: '   ' },
				'edit',
				STORED,
			),
		).toEqual({ secret: 'La clé est obligatoire.' })
	})
})

describe('requiresNewSecret', () => {
	const unchanged = valuesOf(STORED)

	it('does not when nothing sensitive changed', () => {
		expect(requiresNewSecret(unchanged, STORED)).toBe(false)
		expect(
			requiresNewSecret(
				{ ...unchanged, name: 'Autre', environment: 'staging' },
				STORED,
			),
		).toBe(false)
	})

	it('does when the scheme, the host or the port of the address changes', () => {
		for (const baseUrl of [
			'http://acme.odoo.com',
			'https://other.odoo.com',
			'https://acme.odoo.com:8443',
		]) {
			expect(requiresNewSecret({ ...unchanged, baseUrl }, STORED)).toBe(true)
		}
	})

	it('does not when only the path, the query or the case of the host changes', () => {
		for (const baseUrl of [
			'https://acme.odoo.com/odoo',
			' https://acme.odoo.com/a/b?x=1 ',
			'https://ACME.odoo.com',
			'https://acme.odoo.com:443',
		]) {
			expect(requiresNewSecret({ ...unchanged, baseUrl }, STORED)).toBe(false)
		}
	})

	it('counts an address that does not parse as changed', () => {
		expect(requiresNewSecret({ ...unchanged, baseUrl: 'acme' }, STORED)).toBe(
			true,
		)
		expect(requiresNewSecret({ ...unchanged, baseUrl: '' }, STORED)).toBe(true)
	})

	it('does when the base or the login changes', () => {
		expect(requiresNewSecret({ ...unchanged, database: 'other' }, STORED)).toBe(
			true,
		)
		expect(requiresNewSecret({ ...unchanged, login: 'bot' }, STORED)).toBe(true)
		expect(
			requiresNewSecret({ ...unchanged, database: ' acme ' }, STORED),
		).toBe(false)
	})

	it('ignores the base and the login the kind does not use', () => {
		const n8n = { ...STORED, kind: 'n8n' as const, database: null, login: null }
		expect(
			requiresNewSecret({ ...valuesOf(n8n), database: 'x', login: 'y' }, n8n),
		).toBe(false)
		const json2 = { ...STORED, kind: 'odoo_json2' as const, login: null }
		expect(requiresNewSecret({ ...valuesOf(json2), login: 'y' }, json2)).toBe(
			false,
		)
		expect(requiresNewSecret({ ...valuesOf(json2), database: '' }, json2)).toBe(
			true,
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
