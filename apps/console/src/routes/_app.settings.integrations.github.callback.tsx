import { createFileRoute } from '@tanstack/react-router'
import { GithubCallbackPage } from '#/domain/integrations/feature/github-callback-page'

function asText(value: unknown): string | undefined {
	if (typeof value === 'string' && value !== '') return value
	if (typeof value === 'number') return String(value)
	return undefined
}

export const Route = createFileRoute(
	'/_app/settings/integrations/github/callback',
)({
	validateSearch: (search: Record<string, unknown>) => ({
		installation_id: asText(search.installation_id),
		code: asText(search.code),
		state: asText(search.state),
	}),
	component: GithubCallbackRoute,
})

function GithubCallbackRoute() {
	const { installation_id, code, state } = Route.useSearch()
	return (
		<GithubCallbackPage
			installationId={installation_id}
			code={code}
			state={state}
		/>
	)
}
