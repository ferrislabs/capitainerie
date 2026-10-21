const TOOL_LABELS: ReadonlyMap<string, string> = new Map([
	['odoo_list_models', 'Odoo · lister les modèles'],
	['odoo_describe_model', 'Odoo · décrire un modèle'],
	['odoo_search_read', 'Odoo · rechercher'],
	['odoo_read', 'Odoo · lire'],
	['odoo_count', 'Odoo · compter'],
	['odoo_read_attachment', 'Odoo · lire une pièce jointe'],
	['odoo_create', 'Odoo · créer'],
	['odoo_write', 'Odoo · modifier'],
	['odoo_delete', 'Odoo · supprimer'],
	['odoo_call_method', 'Odoo · appeler une méthode'],
	['n8n_list_workflows', 'n8n · lister les workflows'],
	['n8n_get_workflow', 'n8n · lire un workflow'],
	['n8n_list_workflow_versions', 'n8n · lister les versions d’un workflow'],
	['n8n_list_executions', 'n8n · lister les exécutions'],
	['n8n_get_execution', 'n8n · lire une exécution'],
	['n8n_list_tags', 'n8n · lister les étiquettes'],
	['n8n_list_credentials', 'n8n · lister les identifiants'],
	['n8n_get_workflow_tags', 'n8n · lire les étiquettes d’un workflow'],
	['n8n_create_workflow', 'n8n · créer un workflow'],
	['n8n_update_workflow', 'n8n · modifier un workflow'],
	['n8n_publish_workflow', 'n8n · publier un workflow'],
	['n8n_unpublish_workflow', 'n8n · dépublier un workflow'],
	['n8n_archive_workflow', 'n8n · archiver un workflow'],
	['n8n_unarchive_workflow', 'n8n · désarchiver un workflow'],
	['n8n_set_workflow_tags', 'n8n · définir les étiquettes d’un workflow'],
	['n8n_create_tag', 'n8n · créer une étiquette'],
	['n8n_delete_tag', 'n8n · supprimer une étiquette'],
	['n8n_retry_execution', 'n8n · relancer une exécution'],
	['n8n_stop_execution', 'n8n · arrêter une exécution'],
])

const REASON_LABELS: ReadonlyMap<string, string> = new Map([
	['instance_not_granted', 'instance non autorisée pour cette intervention'],
	['write_not_granted', 'écriture non autorisée sur cette instance'],
	[
		'instance_of_another_client',
		'instance d’un autre client que celui de la carte',
	],
	['instance_archived', 'instance archivée'],
	['sensitive_model', 'modèle Odoo sensible, inaccessible'],
	['tool_not_for_instance', 'outil inadapté à ce type d’instance'],
	['policy_denied', 'refusé par la politique d’accès'],
	[
		'masked_value_without_original',
		'valeur masquée dont l’original n’est pas connu',
	],
	['not_supported', 'non pris en charge par cette instance'],
	[
		'before_unreadable',
		'l’état d’avant n’a pas pu être lu : rien n’a été écrit',
	],
	['connection_failed', 'connexion à l’instance impossible'],
	['network_blocked', 'adresse bloquée par la garde réseau'],
	['version_mismatch', 'version de l’instance incompatible'],
])

const RESOURCE_LABELS: ReadonlyMap<string, string> = new Map([
	['workflows', 'workflows'],
	['executions', 'exécutions'],
	['tags', 'étiquettes'],
	['credentials', 'identifiants'],
])

const MAX_IDS = 5

export function toolLabel(tool: string): string {
	return TOOL_LABELS.get(tool) ?? tool
}

export function reasonLabel(
	code: string | null | undefined,
	reason: string | null | undefined,
): string | undefined {
	const known = code ? REASON_LABELS.get(code) : undefined
	if (known) return known
	return reason || code || undefined
}

function fieldsOf(target: unknown): Record<string, unknown> | undefined {
	return typeof target === 'object' && target !== null && !Array.isArray(target)
		? (target as Record<string, unknown>)
		: undefined
}

function text(value: unknown): string | undefined {
	return typeof value === 'string' || typeof value === 'number'
		? String(value)
		: undefined
}

function idsLabel(value: unknown): string | undefined {
	if (!Array.isArray(value) || value.length === 0) return undefined
	const shown = value.slice(0, MAX_IDS).map((id) => `#${String(id)}`)
	const hidden = value.length - shown.length
	if (hidden === 0) return shown.join(', ')
	return `${shown.join(', ')} et ${hidden} ${hidden > 1 ? 'autres' : 'autre'}`
}

function modelPart(fields: Record<string, unknown>): string | undefined {
	const model = text(fields.model)
	if (model === undefined) return undefined
	const method = text(fields.method)
	const ids = idsLabel(fields.ids)
	return [method === undefined ? model : `${model} · ${method}`, ids]
		.filter((part) => part !== undefined)
		.join(' ')
}

function attachmentPart(fields: Record<string, unknown>): string | undefined {
	const id = text(fields.attachment_id)
	return id === undefined ? undefined : `pièce jointe #${id}`
}

function prefixed(prefix: string, value: unknown): string | undefined {
	const label = text(value)
	return label === undefined ? undefined : `${prefix} ${label}`
}

function quoted(prefix: string, value: unknown): string | undefined {
	const label = text(value)
	return label === undefined ? undefined : `${prefix} « ${label} »`
}

export function targetSummary(target: unknown): string {
	const fields = fieldsOf(target)
	if (!fields) return '—'
	const resource = text(fields.resource)
	const parts = [
		modelPart(fields),
		attachmentPart(fields),
		resource === undefined
			? undefined
			: (RESOURCE_LABELS.get(resource) ?? resource),
		prefixed('workflow', fields.workflow_id),
		quoted('workflow', fields.workflow_name),
		prefixed('exécution', fields.execution_id),
		quoted('étiquette', fields.tag_name),
		prefixed('étiquette', fields.tag_id),
	].filter((part) => part !== undefined)
	if (parts.length > 0) return parts.join(' · ')
	return Object.keys(fields).length === 0 ? '—' : JSON.stringify(fields)
}
