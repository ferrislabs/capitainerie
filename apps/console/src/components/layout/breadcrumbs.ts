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
	settings: 'Paramètres',
	providers: 'Fournisseurs',
	models: 'Modèles',
	integrations: 'Intégrations',
}

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
		const label = SECTION_LABELS[segment] ?? decodeURIComponent(segment)
		const last = index === segments.length - 1
		crumbs.push(
			last
				? { label }
				: { label, to: `/${segments.slice(0, index + 1).join('/')}` },
		)
	})
	return crumbs
}
