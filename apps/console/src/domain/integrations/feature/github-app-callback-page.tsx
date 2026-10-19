import { useNavigate } from '@tanstack/react-router'
import { useEffect, useRef, useState } from 'react'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { useCreateGithubApp } from '#/domain/integrations/hooks/use-github-mutations'
import { GithubCallbackView } from '#/domain/integrations/ui/github-callback-view'

export const INCOMPLETE_APP_CALLBACK_MESSAGE =
	'Le retour de GitHub est incomplet. Recommencez la création depuis les intégrations.'

interface Props {
	code?: string
	state?: string
}

export function GithubAppCallbackPage({ code, state }: Props) {
	const navigate = useNavigate()
	const { create } = useCreateGithubApp()
	const started = useRef(false)
	const [errorMessage, setErrorMessage] = useState<string>()
	const complete = Boolean(code) && Boolean(state)

	// The request is awaited here rather than watched through the mutation
	// state: React's development remount detaches the mutation observer, and
	// the page then never leaves « en cours ».
	useEffect(() => {
		if (started.current || !code || !state) return
		started.current = true
		void create({ code, state }).then((error) => {
			if (error) {
				setErrorMessage(error)
				return
			}
			void navigate({
				to: '/settings/integrations',
				search: { app: 'created' },
				replace: true,
			})
		})
	}, [code, state, create, navigate])

	return (
		<>
			<PageHeader title="Création de l’application GitHub" />
			<SectionCard title="GitHub">
				<GithubCallbackView
					pendingLabel="Création de l’application GitHub en cours…"
					errorMessage={
						complete ? errorMessage : INCOMPLETE_APP_CALLBACK_MESSAGE
					}
				/>
			</SectionCard>
		</>
	)
}
