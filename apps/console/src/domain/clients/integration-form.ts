import type { Schemas } from '#/api/api.client'
import { validateName } from '#/domain/clients/aliases'

export interface IntegrationValues {
	kind: Schemas.IntegrationKindDto
	name: string
	environment: Schemas.IntegrationEnvironmentDto
	baseUrl: string
	database: string
	login: string
	secret: string
}

export interface IntegrationErrors {
	name?: string
	baseUrl?: string
	database?: string
	login?: string
	secret?: string
}

export type IntegrationMode = 'create' | 'edit'

export const EMPTY_VALUES: IntegrationValues = {
	kind: 'odoo_json2',
	name: '',
	environment: 'production',
	baseUrl: '',
	database: '',
	login: '',
	secret: '',
}

export function usesDatabase(kind: Schemas.IntegrationKindDto): boolean {
	return kind !== 'n8n'
}

export function usesLogin(kind: Schemas.IntegrationKindDto): boolean {
	return kind === 'odoo_jsonrpc'
}

export function valuesOf(
	instance: Schemas.IntegrationInstanceResponse,
): IntegrationValues {
	return {
		kind: instance.kind,
		name: instance.name,
		environment: instance.environment,
		baseUrl: instance.base_url,
		database: instance.database ?? '',
		login: instance.login ?? '',
		secret: '',
	}
}

function originOf(address: string): string | undefined {
	try {
		const url = new URL(address)
		return `${url.protocol}//${url.host}`
	} catch {
		return undefined
	}
}

function originChanged(address: string, stored: string): boolean {
	const next = address.trim()
	if (next === stored.trim()) return false
	const nextOrigin = originOf(next)
	const storedOrigin = originOf(stored.trim())
	return (
		nextOrigin === undefined ||
		storedOrigin === undefined ||
		nextOrigin !== storedOrigin
	)
}

export function requiresNewSecret(
	values: IntegrationValues,
	instance: Schemas.IntegrationInstanceResponse,
): boolean {
	return (
		originChanged(values.baseUrl, instance.base_url) ||
		(usesDatabase(instance.kind) &&
			values.database.trim() !== (instance.database ?? '')) ||
		(usesLogin(instance.kind) && values.login.trim() !== (instance.login ?? ''))
	)
}

export function validateIntegration(
	values: IntegrationValues,
	mode: IntegrationMode,
	instance?: Schemas.IntegrationInstanceResponse,
): IntegrationErrors {
	const errors: IntegrationErrors = {}
	const name = validateName(values.name)
	if (name) errors.name = name
	if (values.baseUrl.trim() === '')
		errors.baseUrl = 'L’adresse est obligatoire.'
	if (values.kind === 'odoo_jsonrpc') {
		if (values.database.trim() === '') {
			errors.database = 'La base est obligatoire.'
		}
		if (values.login.trim() === '') {
			errors.login = 'L’identifiant est obligatoire.'
		}
	}
	const secretRequired =
		mode === 'create' ||
		(instance !== undefined && requiresNewSecret(values, instance))
	if (secretRequired && values.secret.trim() === '') {
		errors.secret = 'La clé est obligatoire.'
	}
	return errors
}

export function integrationBody(
	values: IntegrationValues,
): Schemas.CreateIntegrationInstanceRequest {
	const database = values.database.trim()
	const login = values.login.trim()
	return {
		kind: values.kind,
		name: values.name.trim(),
		environment: values.environment,
		base_url: values.baseUrl.trim(),
		secret: values.secret,
		...(usesDatabase(values.kind) && database ? { database } : {}),
		...(usesLogin(values.kind) && login ? { login } : {}),
	}
}

export function integrationPatch(
	values: IntegrationValues,
	instance: Schemas.IntegrationInstanceResponse,
): Schemas.UpdateIntegrationInstanceRequest {
	const name = values.name.trim()
	const baseUrl = values.baseUrl.trim()
	const database = values.database.trim()
	const login = values.login.trim()
	return {
		...(name !== instance.name ? { name } : {}),
		...(values.environment !== instance.environment
			? { environment: values.environment }
			: {}),
		...(baseUrl !== instance.base_url ? { base_url: baseUrl } : {}),
		...(usesDatabase(instance.kind) && database !== (instance.database ?? '')
			? { database }
			: {}),
		...(usesLogin(instance.kind) && login !== (instance.login ?? '')
			? { login }
			: {}),
		...(values.secret.trim() !== '' ? { secret: values.secret } : {}),
	}
}
