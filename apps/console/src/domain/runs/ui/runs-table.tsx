import { Play } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { Badge } from '#/components/ui/badge'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'

type Status = Schemas.RunStatusResponse
type BadgeVariant = 'muted' | 'default' | 'success' | 'destructive'

const STATUS_LABELS: Record<Status, string> = {
	queued: 'En attente',
	running: 'En cours',
	succeeded: 'Terminé',
	failed: 'Échec',
	cancelled: 'Annulé',
}

const STATUS_VARIANTS: Record<Status, BadgeVariant> = {
	queued: 'muted',
	running: 'default',
	succeeded: 'success',
	failed: 'destructive',
	cancelled: 'muted',
}

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

interface Props {
	runs: ReadonlyArray<Schemas.RunResponse>
}

export function RunsTable({ runs }: Props) {
	if (runs.length === 0) {
		return (
			<EmptyState
				icon={Play}
				title="Aucun run"
				description="Déposez un transcript pour lancer un premier run."
			/>
		)
	}

	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Titre</TableHead>
					<TableHead>Statut</TableHead>
					<TableHead>Lancé le</TableHead>
					<TableHead>Étapes</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{runs.map((run) => (
					<TableRow key={run.id}>
						<TableCell className="font-medium">
							{run.transcript_title}
						</TableCell>
						<TableCell>
							<Badge variant={STATUS_VARIANTS[run.status]}>
								{STATUS_LABELS[run.status]}
							</Badge>
						</TableCell>
						<TableCell>{dateFormat.format(new Date(run.created_at))}</TableCell>
						<TableCell>{run.pipeline.join(' → ')}</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}
