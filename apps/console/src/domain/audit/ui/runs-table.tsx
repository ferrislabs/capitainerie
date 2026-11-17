import { SectionCard } from '#/components/layout/section-card'
import { StatusBadge, type StatusTone } from '#/components/layout/status-badge'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { AuditRun, AuditRunStatus } from '#/domain/audit/mock/audit-data'
import { Avatar } from '#/domain/audit/ui/avatar'
import { shortName } from '#/domain/demo'

interface Props {
	runs: ReadonlyArray<AuditRun>
}

const TONES: Record<AuditRunStatus, StatusTone> = {
	running: 'running',
	success: 'success',
	danger: 'danger',
}

export function RunsTable({ runs }: Props) {
	return (
		<SectionCard title="Exécutions récentes" bare>
			<div className="overflow-hidden rounded-lg border bg-card">
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>Agent</TableHead>
							<TableHead>Déclencheur</TableHead>
							<TableHead>Statut</TableHead>
							<TableHead>Durée</TableHead>
							<TableHead>Quand</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{runs.map((run) => (
							<TableRow key={`${run.agent}-${run.when}-${run.trigger}`}>
								<TableCell>
									<span className="inline-flex items-center gap-2">
										<Avatar name={run.agent} size={18} />
										{shortName(run.agent)}
									</span>
								</TableCell>
								<TableCell className="text-muted-foreground">
									{run.trigger}
								</TableCell>
								<TableCell>
									<StatusBadge
										tone={TONES[run.status]}
										label={run.statusLabel}
									/>
								</TableCell>
								<TableCell className="figures">{run.duration}</TableCell>
								<TableCell className="text-muted-foreground">
									{run.when}
								</TableCell>
							</TableRow>
						))}
					</TableBody>
				</Table>
			</div>
		</SectionCard>
	)
}
