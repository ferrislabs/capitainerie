export interface Crumb {
	label: string
	to?: string
	tag?: string
}

export interface EntityNames {
	task: (id: string) => string | undefined
	agent: (id: string) => string | undefined
	source: (id: string) => string | undefined
	routine: (id: string) => string | undefined
}

const SECTION_LABELS: Readonly<Record<string, string>> = {
	dashboard: 'Tableau de bord',
	inbox: 'Boîte de réception',
	tasks: 'Tâches',
	projects: 'Projets',
	routines: 'Routines',
	artifacts: 'Artifacts',
	agents: 'Agents',
	skills: 'Compétences',
	sources: 'Sources',
	connectors: 'Connecteurs',
	approvals: 'Approbations',
	audit: 'Audit',
}

const NEW_LABELS: Readonly<Record<string, string>> = {
	routines: 'Nouvelle routine',
	sources: 'Nouvelle source',
	agents: 'Nouvel agent',
}

const NEW_SEGMENTS: Readonly<Record<string, string>> = {
	routines: 'nouvelle',
	sources: 'nouvelle',
	agents: 'nouveau',
}

const LIST_VIEWS: Readonly<Record<string, string>> = {
	agents: 'organigramme',
}

const HOME = 'Tableau de bord'

const NO_NAMES: EntityNames = {
	task: () => undefined,
	agent: () => undefined,
	source: () => undefined,
	routine: () => undefined,
}

function decode(segment: string): string {
	try {
		return decodeURIComponent(segment)
	} catch {
		return segment
	}
}

/**
 * Fil d'Ariane de la barre du haut, après l'organisation : la page seule pour une
 * section, la section puis le nom de l'objet pour une fiche (tâche, agent, source, routine).
 */
export function breadcrumbs(
	pathname: string,
	names: EntityNames = NO_NAMES,
): ReadonlyArray<Crumb> {
	const [section, id] = pathname.split('/').filter(Boolean)
	if (!section) return [{ label: HOME }]
	const label = SECTION_LABELS[section] ?? decode(section)
	if (!id) return [{ label }]
	if (id === LIST_VIEWS[section]) return [{ label }]
	const parent: Crumb = { label, to: `/${section}` }
	if (id === NEW_SEGMENTS[section]) {
		return [parent, { label: NEW_LABELS[section] ?? decode(id) }]
	}
	const raw = decode(id)
	if (section === 'tasks') {
		const taskId = raw.toUpperCase()
		return [parent, { label: names.task(taskId) ?? taskId, tag: taskId }]
	}
	const lookup =
		section === 'agents'
			? names.agent
			: section === 'sources'
				? names.source
				: section === 'routines'
					? names.routine
					: undefined
	if (!lookup) return [{ label }]
	return [parent, { label: lookup(raw) ?? raw }]
}
