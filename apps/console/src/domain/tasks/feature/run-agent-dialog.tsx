import { Link } from '@tanstack/react-router'
import { useId, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { Checkbox } from '#/components/ui/checkbox'
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '#/components/ui/dialog'
import { Field } from '#/components/ui/field'
import { Input } from '#/components/ui/input'
import { NativeSelect } from '#/components/ui/native-select'
import { RadioGroup, RadioGroupItem } from '#/components/ui/radio-group'
import {
	useRefreshCard,
	useRunCardAgent,
} from '#/domain/tasks/hooks/use-agent-run'
import {
	describeRunError,
	initialRunPlan,
	type RunErrorInfo,
	type RunPlan,
	type RunTarget,
	runRequest,
	writesHint,
} from '#/domain/tasks/run-agent'

interface Props {
	cardId: string
	version: number
	agentId: string
	agentName: string
	/** Dépôts `owner/name` que l'agent a le droit de modifier. */
	repositoryGrants: ReadonlyArray<string>
	hasConnectors: boolean
	/** Titre et description de la tâche, pour présélectionner le dépôt qu'ils nomment. */
	taskText: string
	open: boolean
	onOpenChange: (open: boolean) => void
	onStarted: (runId: string) => void
}

function FailureLinks({
	failure,
	agentId,
}: {
	failure: RunErrorInfo
	agentId: string
}) {
	switch (failure.kind) {
		case 'not_connected':
			return (
				<Link
					to="/connectors/{-$tab}"
					params={{ tab: 'installes' }}
					className="underline"
				>
					Ouvrir les connecteurs
				</Link>
			)
		case 'no_provider':
			return (
				<Link to="/providers" className="underline">
					Connecter un fournisseur
				</Link>
			)
		case 'github':
			return (
				<Link to="/settings/integrations/github" className="underline">
					Ouvrir les réglages GitHub
				</Link>
			)
		case 'agent_repositories':
			return (
				<Link
					to="/agents/$agentId/{-$tab}"
					params={{ agentId, tab: 'connectors' }}
					className="underline"
				>
					Ouvrir les accès de l'agent
				</Link>
			)
		default:
			return null
	}
}

export function RunAgentDialog({
	cardId,
	version,
	agentId,
	agentName,
	repositoryGrants,
	hasConnectors,
	taskText,
	open,
	onOpenChange,
	onStarted,
}: Props) {
	const repositoryId = useId()
	const fresh = () => initialRunPlan(repositoryGrants, hasConnectors, taskText)
	const [plan, setPlan] = useState<RunPlan>(fresh)
	const run = useRunCardAgent()
	const refresh = useRefreshCard()
	const failure = run.isError ? describeRunError(run.error) : null
	const onRepository = plan.target === 'repository'
	const withRepositories = repositoryGrants.length > 0

	const patch = (change: Partial<RunPlan>) => {
		run.reset()
		setPlan((current) => ({ ...current, ...change }))
	}

	const change = (next: boolean) => {
		if (!next) {
			run.reset()
			setPlan(fresh())
		}
		onOpenChange(next)
	}

	const confirm = () =>
		run.mutate(
			{ path: { card_id: cardId }, body: runRequest(plan, version) },
			{
				onSuccess: (data) => {
					change(false)
					onStarted(data.run_id)
				},
				onError: (error) => {
					if (describeRunError(error).stale) refresh()
				},
			},
		)

	return (
		<Dialog open={open} onOpenChange={change}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>Exécuter avec {agentName}</DialogTitle>
					<DialogDescription>
						{onRepository
							? "L'agent travaille dans un environnement isolé, sur une copie du dépôt choisi."
							: "L'agent agira avec ta connexion aux connecteurs qui lui sont accordés."}
					</DialogDescription>
				</DialogHeader>
				{withRepositories ? (
					<Field label="Où travailler" group>
						<RadioGroup
							value={plan.target}
							onValueChange={(value) => patch({ target: value as RunTarget })}
						>
							<RadioGroupItem
								value="connectors"
								label="Connecteurs MCP"
								description={
									hasConnectors
										? 'Les connecteurs accordés à l’agent.'
										: 'Cet agent n’a aucun connecteur accordé.'
								}
							/>
							<RadioGroupItem
								value="repository"
								label="Un dépôt GitHub"
								description="Un des dépôts accordés à l’agent."
							/>
						</RadioGroup>
					</Field>
				) : null}
				{withRepositories && onRepository ? (
					<div className="grid gap-3 sm:grid-cols-2">
						<Field label="Dépôt" id={repositoryId}>
							<NativeSelect
								id={repositoryId}
								className="font-mono"
								value={plan.repository}
								onChange={(event) => patch({ repository: event.target.value })}
							>
								{repositoryGrants.map((name) => (
									<option key={name} value={name}>
										{name}
									</option>
								))}
							</NativeSelect>
						</Field>
						<Field
							label="Branche de base"
							optional
							hint="Vide : la branche par défaut du dépôt."
						>
							<Input
								className="font-mono"
								placeholder="main"
								autoComplete="off"
								value={plan.baseBranch}
								onChange={(event) => patch({ baseBranch: event.target.value })}
							/>
						</Field>
					</div>
				) : null}
				<Checkbox
					label="Autoriser les écritures"
					description={writesHint(plan)}
					checked={plan.allowWrites}
					onCheckedChange={(value) => patch({ allowWrites: value === true })}
				/>
				{failure ? (
					<InlineAlert tone="danger">
						{failure.message}{' '}
						<FailureLinks failure={failure} agentId={agentId} />
					</InlineAlert>
				) : null}
				<DialogFooter>
					<Button variant="outline" onClick={() => change(false)}>
						Annuler
					</Button>
					<Button onClick={confirm} disabled={run.isPending}>
						{run.isPending ? 'Lancement…' : 'Confirmer'}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}
