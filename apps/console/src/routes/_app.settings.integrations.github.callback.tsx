import { createFileRoute } from '@tanstack/react-router'
import { GithubInstallCallbackPage } from '#/domain/integrations/feature/github-callback-pages'

/** Le routeur lit `?code=123` comme un nombre : on le reprend en texte. */
function text(value: unknown): string | undefined {
	if (typeof value === 'number') return String(value)
	return typeof value === 'string' && value !== '' ? value : undefined
}

function integer(value: unknown): number | undefined {
	const parsed =
		typeof value === 'number' ? value : Number.parseInt(String(value), 10)
	return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : undefined
}

/**
 * `setup_url` et `callback_urls` du manifeste : GitHub y renvoie après
 * l'installation avec `installation_id`, `setup_action`, `code` et `state`.
 */
export const Route = createFileRoute(
	'/_app/settings/integrations/github/callback',
)({
	validateSearch: (search) => ({
		installation_id: integer(search.installation_id),
		setup_action: text(search.setup_action),
		code: text(search.code),
		state: text(search.state),
	}),
	component: InstallCallbackRoute,
})

function InstallCallbackRoute() {
	const { installation_id, setup_action, code, state } = Route.useSearch()
	return (
		<GithubInstallCallbackPage
			installationId={installation_id}
			setupAction={setup_action}
			code={code}
			state={state}
		/>
	)
}
