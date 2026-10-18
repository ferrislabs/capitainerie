import { useNavigate } from '@tanstack/react-router'
import { useEffect, useRef, useState } from 'react'
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
	const { connect } = useConnectGithub()
	const started = useRef(false)
	const [errorMessage, setErrorMessage] = useState<string>()
	const id = parseInstallationId(installationId)
	const complete = id !== undefined && Boolean(code) && Boolean(state)

	// Awaited, not watched: see GithubAppCallbackPage.
	useEffect(() => {
		if (started.current || id === undefined || !code || !state) return
		started.current = true
		void connect({ installationId: id, code, state }).then((error) => {
			if (error) {
				setErrorMessage(error)
				return
			}
			void navigate({
				to: '/settings/integrations',
				search: { connected: true },
				replace: true,
			})
		})
	}, [id, code, state, connect, navigate])

	return (
		<>
			<PageHeader title="Connexion à GitHub" />
			<SectionCard title="GitHub">
				<GithubCallbackView
					errorMessage={complete ? errorMessage : INCOMPLETE_CALLBACK_MESSAGE}
				/>
			</SectionCard>
		</>
	)
}
