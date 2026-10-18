import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import {
	useGithubIntegration,
	useGithubRepositories,
} from '#/domain/integrations/hooks/use-github-integration'
import { useExecuteCard } from '#/domain/kanban/hooks/use-execute-card'
import {
	ExecuteCardDialog,
	type RepositoriesState,
} from '#/domain/kanban/ui/execute-card-dialog'
import { hasErrorKey } from '#/lib/api-error'

interface Props {
	card: Schemas.CardResponse | undefined
	onClose: () => void
	onStarted: (runId: string) => void
}

export function ExecuteCardFlow({ card, onClose, onStarted }: Props) {
	const open = card !== undefined
	const integration = useGithubIntegration(open)
	const notConnected =
		integration.data !== undefined && !integration.data.installation
	const repositories = useGithubRepositories(open && !notConnected)
	const execute = useExecuteCard()
	const [chosen, setChosen] = useState('')

	function state(): RepositoriesState {
		if (notConnected) return { kind: 'not_connected' }
		if (hasErrorKey(repositories.error, 'github_not_connected')) {
			return { kind: 'not_connected' }
		}
		if (repositories.isError) return { kind: 'error' }
		if (repositories.data) {
			return { kind: 'ready', repositories: repositories.data.repositories }
		}
		return { kind: 'loading' }
	}

	function close() {
		setChosen('')
		execute.reset()
		onClose()
	}

	function submit() {
		if (!card || !chosen) return
		execute.mutate(
			{
				path: { card_id: card.id },
				body: { repository: chosen, expected_version: card.version },
			},
			{
				onSuccess: (response) => {
					setChosen('')
					execute.reset()
					onStarted(response.run_id)
				},
			},
		)
	}

	return (
		<ExecuteCardDialog
			open={open}
			cardTitle={card?.title ?? ''}
			repositories={state()}
			repository={chosen}
			errorMessage={execute.errorMessage}
			isPending={execute.isPending}
			onRepositoryChange={(fullName) => {
				execute.reset()
				setChosen(fullName)
			}}
			onRetry={() => void repositories.refetch()}
			onSubmit={submit}
			onClose={close}
		/>
	)
}
