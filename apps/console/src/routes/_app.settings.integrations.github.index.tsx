import { createFileRoute } from '@tanstack/react-router'
import type { GithubReturn } from '#/domain/integrations/feature/github-callback-pages'
import { GithubPage } from '#/domain/integrations/feature/github-page'

const BANNERS: Record<GithubReturn, string> = {
	app: 'Application GitHub créée. Installe-la maintenant sur GitHub.',
	installation: 'GitHub est connecté à l’organisation.',
}

export const Route = createFileRoute('/_app/settings/integrations/github/')({
	validateSearch: (search): { retour?: GithubReturn } =>
		search.retour === 'app' || search.retour === 'installation'
			? { retour: search.retour }
			: {},
	component: GithubRoute,
})

function GithubRoute() {
	const { retour } = Route.useSearch()
	return <GithubPage banner={retour ? BANNERS[retour] : undefined} />
}
