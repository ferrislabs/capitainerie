import { useNavigate } from '@tanstack/react-router'
import { useEffect, useRef } from 'react'
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
	const creation = useCreateGithubApp()
	const started = useRef(false)
	const complete = Boolean(code) && Boolean(state)

	useEffect(() => {
		if (started.current || !code || !state) return
		started.current = true
		creation.create({ code, state })
	}, [code, state, creation.create])

	useEffect(() => {
		if (creation.isSuccess) {
			void navigate({
				to: '/settings/integrations',
				search: { app: 'created' },
				replace: true,
			})
		}
	}, [creation.isSuccess, navigate])

	return (
		<>
			<PageHeader title="Création de l’application GitHub" />
			<SectionCard title="GitHub">
				<GithubCallbackView
					pendingLabel="Création de l’application GitHub en cours…"
					errorMessage={
						complete ? creation.errorMessage : INCOMPLETE_APP_CALLBACK_MESSAGE
					}
				/>
			</SectionCard>
		</>
	)
}
