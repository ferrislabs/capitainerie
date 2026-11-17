import type { DemoSource, DemoSourceType } from '#/domain/demo'

interface TypeInfo {
	id: DemoSourceType
	label: string
	description: string
	patternsLabel: string
	placeholder: string
	feeds: ReadonlyArray<string>
	template: string
	variable: string
}

export const SOURCE_TYPES: ReadonlyArray<TypeInfo> = [
	{
		id: 'DNS',
		label: 'Enregistrements DNS',
		description:
			"Nouveaux noms d'hôte vus dans les journaux de transparence des certificats et les zones DNS",
		patternsLabel: "Motifs de nom d'hôte",
		placeholder: 'iam.*, auth.*',
		feeds: [
			'Journaux Certificate Transparency',
			'Zones DNS (AFNIC, CZDS)',
			'Passive DNS',
			'Liste de domaines surveillés',
		],
		template:
			"Sous-domaine {hôte} : identifier l'entreprise dépositaire et ses informations légales",
		variable: '{hôte}',
	},
	{
		id: 'Webhook',
		label: 'Webhook entrant',
		description: 'Une URL signée que ton outil appelle à chaque événement',
		patternsLabel: "Types d'événement",
		placeholder: 'message_created',
		feeds: ['POST /hooks/…'],
		template: 'Événement reçu : {valeur}',
		variable: '{valeur}',
	},
	{
		id: 'Mail',
		label: 'Boîte e-mail',
		description: 'Chaque message reçu sur une adresse dédiée',
		patternsLabel: 'Adresses ou expéditeurs',
		placeholder: 'cabinet@natalia.fr',
		feeds: ['IMAP (boîte partagée)'],
		template: 'Mail reçu : {valeur}',
		variable: '{valeur}',
	},
	{
		id: 'RSS',
		label: 'Flux RSS ou Atom',
		description: "Nouvelles entrées d'un flux (BODACC, JORF, blogs)",
		patternsLabel: 'URL du flux',
		placeholder: 'https://www.bodacc.fr/…/rss',
		feeds: ['Interrogation toutes les 15 min'],
		template: 'Nouvelle entrée : {valeur}',
		variable: '{valeur}',
	},
	{
		id: 'Git',
		label: 'GitLab ou GitHub',
		description: 'Push, merge requests, tags, pipelines',
		patternsLabel: 'Événements',
		placeholder: 'merge_request · opened',
		feeds: ['Webhook GitLab'],
		template: 'Événement Git : {valeur}',
		variable: '{valeur}',
	},
	{
		id: 'Odoo',
		label: 'Odoo',
		description: "Création ou modification d'une fiche",
		patternsLabel: 'Modèle et événement',
		placeholder: 'crm.lead · création',
		feeds: ['Webhook Odoo'],
		template: 'Fiche Odoo : {valeur}',
		variable: '{valeur}',
	},
]

export const DEDUPE_OPTIONS = ['Aucun', '1 h', '24 h', '7 j'] as const
export const MODE_OPTIONS = [
	'Une tâche par événement',
	'Une tâche par domaine racine',
	'Regrouper par heure',
] as const

export const typeInfo = (type: DemoSourceType): TypeInfo =>
	SOURCE_TYPES.find((t) => t.id === type) ?? (SOURCE_TYPES[0] as TypeInfo)

export interface SourceForm {
	name: string
	type: DemoSourceType
	patterns: string
	exclusions: string
	feed: string
	dedupe: string
	agentId: string
	mode: string
	template: string
}

function typeDefaults(type: DemoSourceType) {
	const info = typeInfo(type)
	return {
		type,
		patterns: type === 'DNS' ? 'iam.*, auth.*' : info.placeholder,
		exclusions: type === 'DNS' ? '*.getnatalia.com' : '',
		feed: info.feeds[0] ?? '',
		template: info.template,
		mode:
			type === 'DNS'
				? 'Une tâche par domaine racine'
				: 'Une tâche par événement',
	}
}

export function initialForm(agentId: string): SourceForm {
	return { name: '', dedupe: '24 h', agentId, ...typeDefaults('DNS') }
}

export function switchType(form: SourceForm, type: DemoSourceType): SourceForm {
	return {
		...form,
		...typeDefaults(type),
		dedupe: initialForm(form.agentId).dedupe,
	}
}

export function slugify(name: string): string {
	return name
		.normalize('NFD')
		.replace(/[̀-ͯ]/g, '')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
}

export function parseList(value: string): string[] {
	return value
		.split(',')
		.map((z) => z.trim())
		.filter(Boolean)
}

export function validateForm(form: SourceForm): string | null {
	return form.name.trim() && parseList(form.patterns).length > 0
		? null
		: 'Renseigne un nom et au moins un motif'
}

export function exampleTitle(form: SourceForm): string {
	const first = parseList(form.patterns)[0] ?? 'iam.*'
	const sample = first.replace(
		'*',
		form.type === 'DNS' ? 'exemple.fr' : 'exemple',
	)
	return form.template.replace(typeInfo(form.type).variable, sample)
}

export function toSource(
	form: SourceForm,
	existingIds: ReadonlyArray<string>,
): DemoSource {
	const base = slugify(form.name) || 'source'
	let id = base
	for (let n = 2; existingIds.includes(id); n++) id = `${base}-${n}`
	return {
		id,
		name: form.name.trim(),
		type: form.type,
		patterns: parseList(form.patterns),
		exclusions: parseList(form.exclusions),
		feeds: [form.feed],
		scope: '',
		mode: form.mode,
		dedupe: form.dedupe,
		template: form.template,
		agentIds: form.agentId ? [form.agentId] : [],
		enabled: true,
		eventsLast24h: 0,
		last: 'jamais',
	}
}
