import { createFileRoute } from '@tanstack/react-router'
import { GithubAppCallbackPage } from '#/domain/integrations/feature/github-app-callback-page'

function asText(value: unknown): string | undefined {
	return typeof value === 'string' && value !== '' ? value : undefined
}

export const Route = createFileRoute(
	'/_app/settings/integrations/github/app-callback',
)({
	validateSearch: (search: Record<string, unknown>) => ({
		code: asText(search.code),
		state: asText(search.state),
	}),
	component: GithubAppCallbackRoute,
})

function GithubAppCallbackRoute() {
	const { code, state } = Route.useSearch()
	return <GithubAppCallbackPage code={code} state={state} />
}
