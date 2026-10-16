import { Link } from '@tanstack/react-router'
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
import { STATUS_LABELS, STATUS_VARIANTS } from '#/domain/runs/status'

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
							<Link
								to="/runs/$runId"
								params={{ runId: run.id }}
								className="hover:underline"
							>
								{run.transcript_title}
							</Link>
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
