import { createFileRoute } from '@tanstack/react-router'
import { GithubAppCallbackPage } from '#/domain/integrations/feature/github-callback-pages'

/** Le routeur lit `?code=123` comme un nombre : on le reprend en texte. */
function text(value: unknown): string | undefined {
	if (typeof value === 'number') return String(value)
	return typeof value === 'string' && value !== '' ? value : undefined
}

/** `redirect_url` du manifeste : GitHub y renvoie avec `code` et `state`. */
export const Route = createFileRoute(
	'/_app/settings/integrations/github/app-callback',
)({
	validateSearch: (search) => ({
		code: text(search.code),
		state: text(search.state),
		error: text(search.error),
	}),
	component: AppCallbackRoute,
})

function AppCallbackRoute() {
	const { code, state, error } = Route.useSearch()
	return <GithubAppCallbackPage code={code} state={state} error={error} />
}
