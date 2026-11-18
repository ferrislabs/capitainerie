import { createFileRoute } from '@tanstack/react-router'
import { OAuthCallbackPage } from '#/domain/connectors/feature/oauth-callback-page'

const text = (value: unknown) =>
	typeof value === 'string' && value !== '' ? value : undefined

export const Route = createFileRoute('/_app/connectors/oauth/callback')({
	validateSearch: (search) => ({
		code: text(search.code),
		state: text(search.state),
		error: text(search.error),
		error_description: text(search.error_description),
	}),
	component: CallbackRoute,
})

function CallbackRoute() {
	const { code, state, error, error_description } = Route.useSearch()
	return (
		<OAuthCallbackPage
			code={code}
			state={state}
			error={error}
			errorDescription={error_description}
		/>
	)
}
