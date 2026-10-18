import { useNavigate } from '@tanstack/react-router'
import { useEffect, useRef } from 'react'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { useConnectGithub } from '#/domain/integrations/hooks/use-github-mutations'
import { GithubCallbackView } from '#/domain/integrations/ui/github-callback-view'

export const INCOMPLETE_CALLBACK_MESSAGE =
	'Le retour de GitHub est incomplet. Recommencez la connexion depuis les intégrations.'

interface Props {
	installationId?: string
	code?: string
	state?: string
}

function parseInstallationId(value: string | undefined): number | undefined {
	if (!value || !/^\d+$/.test(value)) return undefined
	const parsed = Number(value)
	return Number.isSafeInteger(parsed) ? parsed : undefined
}

export function GithubCallbackPage({ installationId, code, state }: Props) {
	const navigate = useNavigate()
	const connection = useConnectGithub()
	const started = useRef(false)
	const id = parseInstallationId(installationId)
	const complete = id !== undefined && Boolean(code) && Boolean(state)

	useEffect(() => {
		if (started.current || id === undefined || !code || !state) return
		started.current = true
		connection.connect({ installationId: id, code, state })
	}, [id, code, state, connection.connect])

	useEffect(() => {
		if (connection.isSuccess) {
			void navigate({
				to: '/settings/integrations',
				search: { connected: true },
				replace: true,
			})
		}
	}, [connection.isSuccess, navigate])

	return (
		<>
			<PageHeader title="Connexion à GitHub" />
			<SectionCard title="GitHub">
				<GithubCallbackView
					errorMessage={
						complete ? connection.errorMessage : INCOMPLETE_CALLBACK_MESSAGE
					}
				/>
			</SectionCard>
		</>
	)
}
