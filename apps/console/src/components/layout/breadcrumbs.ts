export interface Crumb {
	label: string
	to?: string
}

const SECTION_LABELS: Readonly<Record<string, string>> = {
	chat: 'Conversations',
	runs: 'Runs',
	live: 'En cours',
	kanban: 'Kanban',
	clients: 'Clients',
	projects: 'Projets',
	agents: 'Agents',
	sources: 'Sources',
	routines: 'Routines',
	tasks: 'Tâches',
	inbox: 'Boîte de réception',
	artifacts: 'Artifacts',
	skills: 'Compétences',
	connectors: 'Connecteurs',
	approvals: 'Approbations',
	audit: 'Audit',
	nouvelle: 'Nouvelle',
	equipe: 'Équipe',
	organigramme: 'Organigramme',
	nouveau: 'Nouvel agent',
	settings: 'Paramètres',
	providers: 'Fournisseurs',
	models: 'Modèles',
	integrations: 'Intégrations',
}

const TAB_LABELS: Readonly<Record<string, string>> = {
	apercu: "Vue d'ensemble",
	overview: "Vue d'ensemble",
	regles: 'Motifs et exclusions',
	pipeline: 'Pipeline',
	evenements: 'Événements',
	declencheurs: 'Déclencheurs',
	executions: 'Exécutions',
	reglages: 'Réglages',
	agents: 'Agents',
	instructions: 'Instructions',
	skills: 'Compétences',
	sources: 'Sources',
	team: 'Délégation',
	connectors: 'Connecteurs',
	runtime: 'Runtime',
	secrets: 'Secrets et variables',
	perms: 'Permissions',
	keys: 'Clés API',
	rev: 'Révisions',
	activite: 'Activité',
	runs: 'Exécutions',
	couts: 'Coûts',
	budgets: 'Budgets',
	chronologie: 'Chronologie',
	tous: 'Tous',
	moi: 'Moi',
	recent: 'Récent',
	'non-lus': 'Non lus',
	installes: 'Installés',
	canaux: 'Canaux',
	outils: 'Outils',
	registres: 'Registres',
}

const TABBED_SECTIONS: ReadonlySet<string> = new Set([
	'audit',
	'inbox',
	'connectors',
])

const HOME: Crumb = { label: 'Accueil', to: '/' }

/**
 * Fil d'Ariane déduit du chemin : un maillon par segment connu, le dernier
 * sans lien. Un segment inconnu (identifiant) garde son texte tel quel.
 */
export function breadcrumbs(pathname: string): ReadonlyArray<Crumb> {
	const segments = pathname.split('/').filter(Boolean)
	if (segments.length === 0) return [{ label: HOME.label }]
	const crumbs: Crumb[] = [HOME]
	segments.forEach((segment, index) => {
		const label =
			(index >= 2 || (index === 1 && TABBED_SECTIONS.has(segments[0] ?? ''))
				? TAB_LABELS[segment]
				: undefined) ??
			SECTION_LABELS[segment] ??
			decodeURIComponent(segment)
		const last = index === segments.length - 1
		crumbs.push(
			last
				? { label }
				: { label, to: `/${segments.slice(0, index + 1).join('/')}` },
		)
	})
	return crumbs
}
