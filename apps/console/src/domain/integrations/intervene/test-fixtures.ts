import type { Schemas } from '#/api/api.client'

export const ODOO_PROD: Schemas.IntegrationInstanceResponse = {
	id: 'i1',
	client_id: 'cl1',
	kind: 'odoo_json2',
	name: 'Odoo prod',
	environment: 'production',
	base_url: 'https://acme.odoo.com',
	database: null,
	login: null,
	secret_preview: '…c3d4',
	server_version: '19.0',
	checked_at: null,
	created_by: 'ada',
	created_at: '2026-10-16T09:00:00Z',
	updated_at: '2026-10-16T09:00:00Z',
	archived_at: null,
}

export const ODOO_STAGING: Schemas.IntegrationInstanceResponse = {
	...ODOO_PROD,
	id: 'i2',
	kind: 'odoo_jsonrpc',
	name: 'Odoo recette',
	environment: 'staging',
	base_url: 'https://recette.acme.test',
	database: 'acme',
	login: 'api@acme.test',
	server_version: '17.0',
}

export const N8N_DEV: Schemas.IntegrationInstanceResponse = {
	...ODOO_PROD,
	id: 'i3',
	kind: 'n8n',
	name: 'Flux',
	environment: 'development',
	base_url: 'https://n8n.acme.test',
	server_version: '1',
}

export const ODOO_OLD: Schemas.IntegrationInstanceResponse = {
	...ODOO_STAGING,
	id: 'i4',
	name: 'Odoo ancien',
	archived_at: '2026-10-18T09:00:00Z',
}

export const TODO_CARD: Schemas.CardResponse = {
	id: 'c1',
	title: 'Corriger le bug',
	description: '',
	column: 'todo',
	provenance: null,
	created_by_run: null,
	created_by_subject: 'ada',
	created_by_agent: null,
	repository_hint: null,
	client: { id: 'cl1', name: 'Acme' },
	version: 5,
	created_at: '2026-10-16T09:30:00Z',
	updated_at: '2026-10-16T09:30:00Z',
	acceptance_criteria: [],
	priority: 'normal',
	labels: [],
}
