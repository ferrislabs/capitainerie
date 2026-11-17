import type { AuditRun, AuditRunStatus } from '#/domain/audit/mock/audit-data'
import { AgentAvatar } from '#/domain/audit/ui/avatar'
import { DataTable, Td, Th, Tr } from '#/domain/audit/ui/data-table'
import { Sev, type SevTone } from '#/domain/audit/ui/sev'
import { shortName } from '#/domain/demo'

interface Props {
	runs: ReadonlyArray<AuditRun>
}

const TONES: Record<AuditRunStatus, SevTone> = {
	running: 'vio',
	success: 'ok',
	danger: 'crit',
}

export function RunsTable({ runs }: Props) {
	return (
		<DataTable>
			<thead>
				<tr>
					<Th>Agent</Th>
					<Th>Déclencheur</Th>
					<Th>Statut</Th>
					<Th>Durée</Th>
					<Th>Quand</Th>
				</tr>
			</thead>
			<tbody>
				{runs.map((run) => (
					<Tr key={`${run.agent}-${run.when}-${run.trigger}`}>
						<Td>
							<span className="flex items-center gap-1.5 text-xs whitespace-nowrap">
								<AgentAvatar name={run.agent} size={18} />
								{shortName(run.agent)}
							</span>
						</Td>
						<Td className="text-base text-muted-foreground">{run.trigger}</Td>
						<Td>
							<Sev tone={TONES[run.status]}>{run.statusLabel}</Sev>
						</Td>
						<Td className="figures font-mono text-[11.5px]">{run.duration}</Td>
						<Td className="text-base text-muted-foreground">{run.when}</Td>
					</Tr>
				))}
			</tbody>
		</DataTable>
	)
}
