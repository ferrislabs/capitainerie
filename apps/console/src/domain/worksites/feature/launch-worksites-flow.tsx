import { useState } from 'react'
import {
	useGithubIntegration,
	useGithubIssues,
	useGithubRepositories,
} from '#/domain/integrations/hooks/use-github'
import { useLaunchWorksites } from '#/domain/worksites/hooks/use-worksites'
import {
	type IssuesState,
	LaunchWorksitesDialog,
	type RepositoriesState,
} from '#/domain/worksites/ui/launch-worksites-dialog'
import { issuesErrorMessage } from '#/domain/worksites/worksite-errors'
import { hasErrorKey } from '#/lib/api-error'

interface Props {
	open: boolean
	project: { id: string; name: string }
	defaultRepository: string | null
	onClose: () => void
	onLaunched: (count: number) => void
}

export function LaunchWorksitesFlow({
	open,
	project,
	defaultRepository,
	onClose,
	onLaunched,
}: Props) {
	const integration = useGithubIntegration(open)
	const notConnected =
		integration.data !== undefined && !integration.data.installation
	const repositories = useGithubRepositories(open && !notConnected)
	const launch = useLaunchWorksites()
	const [chosen, setChosen] = useState('')
	const [prompts, setPrompts] = useState<string[]>([''])
	const [selectedIssues, setSelectedIssues] = useState<number[]>([])

	const hintAvailable =
		defaultRepository !== null &&
		(repositories.data?.repositories.some(
			(item) => item.full_name === defaultRepository,
		) ??
			false)
	const repository =
		chosen !== '' ? chosen : hintAvailable ? defaultRepository : ''
	const issues = useGithubIssues(open ? repository : '')

	function repositoriesState(): RepositoriesState {
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

	function issuesState(): IssuesState {
		if (repository === '') return { kind: 'idle' }
		if (issues.isError) {
			return { kind: 'error', message: issuesErrorMessage(issues.error) }
		}
		if (issues.data) return { kind: 'ready', issues: issues.data.issues }
		return { kind: 'loading' }
	}

	function reset() {
		setChosen('')
		setPrompts([''])
		setSelectedIssues([])
		launch.reset()
	}

	function close() {
		reset()
		onClose()
	}

	function submit() {
		if (repository === '') return
		const filled = prompts.filter((prompt) => prompt.trim() !== '')
		launch.mutate(
			{
				path: { project_id: project.id },
				body: { repository, prompts: filled, issues: selectedIssues },
			},
			{
				onSuccess: (result) => {
					reset()
					onLaunched(result.worksites.length)
				},
			},
		)
	}

	return (
		<LaunchWorksitesDialog
			open={open}
			projectName={project.name}
			repositories={repositoriesState()}
			repository={repository}
			prompts={prompts}
			issues={issuesState()}
			selectedIssues={selectedIssues}
			errorMessage={launch.errorMessage}
			isPending={launch.isPending}
			onRepositoryChange={(next) => {
				setChosen(next)
				setSelectedIssues([])
			}}
			onPromptChange={(index, value) =>
				setPrompts((current) =>
					current.map((prompt, i) => (i === index ? value : prompt)),
				)
			}
			onAddPrompt={() => setPrompts((current) => [...current, ''])}
			onRemovePrompt={(index) =>
				setPrompts((current) => current.filter((_, i) => i !== index))
			}
			onToggleIssue={(number, selected) =>
				setSelectedIssues((current) =>
					selected
						? current.includes(number)
							? current
							: [...current, number]
						: current.filter((item) => item !== number),
				)
			}
			onRetry={() => void repositories.refetch()}
			onSubmit={submit}
			onClose={close}
		/>
	)
}
