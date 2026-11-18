import { Link } from '@tanstack/react-router'
import { useState } from 'react'
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
import {
	useRefreshCard,
	useRunCardAgent,
} from '#/domain/tasks/hooks/use-agent-run'
import { describeRunError } from '#/domain/tasks/run-agent'

interface Props {
	cardId: string
	version: number
	agentName: string
	open: boolean
	onOpenChange: (open: boolean) => void
	onStarted: (runId: string) => void
}

export function RunAgentDialog({
	cardId,
	version,
	agentName,
	open,
	onOpenChange,
	onStarted,
}: Props) {
	const [allowWrites, setAllowWrites] = useState(false)
	const run = useRunCardAgent()
	const refresh = useRefreshCard()
	const failure = run.isError ? describeRunError(run.error) : null

	const change = (next: boolean) => {
		if (!next) {
			run.reset()
			setAllowWrites(false)
		}
		onOpenChange(next)
	}

	const confirm = () =>
		run.mutate(
			{
				path: { card_id: cardId },
				body: { expected_version: version, allow_writes: allowWrites },
			},
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
						L'agent agira avec ta connexion aux connecteurs qui lui sont
						accordés.
					</DialogDescription>
				</DialogHeader>
				<Checkbox
					label="Autoriser les écritures"
					description="Les écritures modifient les données du service connecté."
					checked={allowWrites}
					onCheckedChange={(value) => setAllowWrites(value === true)}
				/>
				{failure ? (
					<InlineAlert tone="danger">
						{failure.message}
						{failure.kind === 'not_connected' ? (
							<>
								{' '}
								<Link
									to="/connectors/{-$tab}"
									params={{ tab: 'installes' }}
									className="underline"
								>
									Ouvrir les connecteurs
								</Link>
							</>
						) : null}
						{failure.kind === 'no_provider' ? (
							<>
								{' '}
								<Link to="/providers" className="underline">
									Connecter un fournisseur
								</Link>
							</>
						) : null}
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
