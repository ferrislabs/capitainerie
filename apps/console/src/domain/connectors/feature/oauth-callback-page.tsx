import { useNavigate } from '@tanstack/react-router'
import { Loader2 } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { useCompleteMcpOAuth } from '#/domain/connectors/hooks/use-mcp-connectors'
import { describeCallbackError } from '#/domain/connectors/model'

interface Props {
	code: string | undefined
	state: string | undefined
	error: string | undefined
	errorDescription?: string | undefined
}

export function OAuthCallbackPage({
	code,
	state,
	error,
	errorDescription,
}: Props) {
	const navigate = useNavigate()
	const complete = useCompleteMcpOAuth()
	const submitted = useRef(false)
	const { mutate } = complete

	const problem = error
		? `Autorisation refusée${errorDescription ? ` : ${errorDescription}` : ` (${error})`}.`
		: !code || !state
			? "Retour d'autorisation incomplet : le code ou l'état est manquant."
			: null

	useEffect(() => {
		if (problem || !code || !state || submitted.current) return
		submitted.current = true
		mutate(
			{ body: { code, state } },
			{
				onSuccess: (result) =>
					void navigate({
						to: '/connectors/{-$tab}',
						params: { tab: 'installes' },
						search: { connected: result.connector_id },
						replace: true,
					}),
			},
		)
	}, [problem, code, state, mutate, navigate])

	const message =
		problem ?? (complete.isError ? describeCallbackError(complete.error) : null)

	return (
		<Page>
			<PageHeader title="Connexion du connecteur" />
			{message ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">{message}</InlineAlert>
					<div>
						<Button
							variant="outline"
							onClick={() =>
								void navigate({
									to: '/connectors/{-$tab}',
									params: { tab: 'installes' },
								})
							}
						>
							Retour aux connecteurs
						</Button>
					</div>
				</div>
			) : (
				<output className="m-0 flex items-center gap-2 text-muted-foreground">
					<Loader2 aria-hidden="true" className="size-4 animate-spin" />
					Finalisation de la connexion…
				</output>
			)}
		</Page>
	)
}
