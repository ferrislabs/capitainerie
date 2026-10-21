import type { Schemas } from '#/api/api.client'

export const KINDS: ReadonlyArray<Schemas.IntegrationKindDto> = [
	'odoo_json2',
	'odoo_jsonrpc',
	'n8n',
]

export const ENVIRONMENTS: ReadonlyArray<Schemas.IntegrationEnvironmentDto> = [
	'production',
	'staging',
	'development',
]

export const KIND_LABELS: Record<Schemas.IntegrationKindDto, string> = {
	odoo_json2: 'Odoo 19 et plus',
	odoo_jsonrpc: 'Odoo 18 et moins',
	n8n: 'n8n',
}

export const ENVIRONMENT_LABELS: Record<
	Schemas.IntegrationEnvironmentDto,
	string
> = {
	production: 'Production',
	staging: 'Staging',
	development: 'Développement',
}

export const ENVIRONMENT_VARIANTS: Record<
	Schemas.IntegrationEnvironmentDto,
	'danger' | 'attention' | 'neutral'
> = {
	production: 'danger',
	staging: 'attention',
	development: 'neutral',
}

export function serverVersionLabel(
	kind: Schemas.IntegrationKindDto,
	version: string | null | undefined,
): string | undefined {
	if (!version) return undefined
	if (kind === 'n8n' && /^\d+$/.test(version)) return `n8n ${version}.x`
	return version
}
