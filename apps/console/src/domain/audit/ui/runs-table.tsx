import { Link } from '@tanstack/react-router'
import type { Schemas } from '#/api/api.client'
import { runAgent, runDuration, runStatusOf } from '#/domain/audit/model'
import { DataTable, Td, Th, Tr } from '#/domain/audit/ui/data-table'
import { Panel, PanelEmpty } from '#/domain/audit/ui/rows'
import { Sev } from '#/domain/audit/ui/sev'
import { relativeAge } from '#/domain/dashboard/format'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'
import { shortName } from '#/lib/agent-name'

interface Props {
	runs: ReadonlyArray<Schemas.RunResponse>
	hueOf: (id: string | null | undefined, name: string) => number
}

function runTitle(run: Schemas.RunResponse): string {
	return run.title ?? run.transcript_title ?? run.repository ?? 'Exécution'
}

export function RunsTable({ runs, hueOf }: Props) {
	if (runs.length === 0) {
		return (
			<Panel>
				<PanelEmpty title="Aucune exécution">
					Aucune exécution ne correspond à ces filtres.
				</PanelEmpty>
			</Panel>
		)
	}
	return (
		<DataTable>
			<thead>
				<tr>
					<Th>Agent</Th>
					<Th>Exécution</Th>
					<Th>Statut</Th>
					<Th>Durée</Th>
					<Th>Quand</Th>
				</tr>
			</thead>
			<tbody>
				{runs.map((run) => {
					const agent = runAgent(run)
					const status = runStatusOf(run.status)
					return (
						<Tr key={run.id}>
							<Td>
								{agent ? (
									<span className="flex items-center gap-1.5 text-xs whitespace-nowrap">
										<AgentAvatar hue={hueOf(agent.id, agent.name)} size={18} />
										{shortName(agent.name)}
									</span>
								) : (
									<span className="text-xs text-faint">–</span>
								)}
							</Td>
							<Td className="text-muted-foreground">
								{run.card_id ? (
									<Link
										to="/tasks/$taskId"
										params={{ taskId: run.card_id }}
										className="hover:text-foreground hover:underline"
									>
										{runTitle(run)}
									</Link>
								) : (
									runTitle(run)
								)}
								{run.failure ? (
									<span className="block truncate text-xs text-signal-danger">
										{run.failure}
									</span>
								) : null}
							</Td>
							<Td>
								<Sev tone={status.tone}>{status.label}</Sev>
							</Td>
							<Td className="figures font-mono text-[11.5px]">
								{runDuration(run)}
							</Td>
							<Td className="text-muted-foreground whitespace-nowrap">
								{relativeAge(run.created_at)}
							</Td>
						</Tr>
					)
				})}
			</tbody>
		</DataTable>
	)
}
