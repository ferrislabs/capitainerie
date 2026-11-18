import { Link } from '@tanstack/react-router'
import type { Schemas } from '#/api/api.client'
import { relativeAge } from '#/domain/dashboard/format'
import type { Source } from '#/domain/sources/model'
import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'
import { Sev, Sw, Tbl, Td, Th, Tr } from '#/domain/sources/ui/kit'

interface Props {
	sources: ReadonlyArray<Source>
	agents: ReadonlyArray<Schemas.TeamAgentResponse>
	busy: boolean
	onToggle: (source: Source, enabled: boolean) => void
}

export function SourcesTable({ sources, agents, busy, onToggle }: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Source</Th>
					<Th>Agent par défaut</Th>
					<Th>24 h</Th>
					<Th>Dernier événement</Th>
					<Th>Actif</Th>
				</tr>
			</thead>
			<tbody>
				{sources.map((source) => {
					const agent = agents.find((a) => a.id === source.default_agent_id)
					return (
						<Tr key={source.id}>
							<Td>
								<Link
									to="/sources/$sourceId/{-$tab}"
									params={{ sourceId: source.id }}
									className="font-medium text-foreground"
								>
									{source.name}
								</Link>
								<div className="font-mono text-[11.5px] text-muted-foreground">
									{source.slug}
								</div>
							</Td>
							<Td>
								{agent ? (
									<span className="flex min-w-0 items-center gap-1.5 text-[12px] text-muted-foreground">
										<AgentAvatar agent={agent} decorative />
										<span className="truncate">{agent.name}</span>
									</span>
								) : source.default_agent_id ? (
									<span className="text-[12px] text-muted-foreground">
										Agent inconnu
									</span>
								) : (
									<Sev tone="warn">Aucun agent</Sev>
								)}
							</Td>
							<Td className="tabular-nums">{source.events_last_24h}</Td>
							<Td className="whitespace-nowrap text-muted-foreground">
								{source.last_event_at ? (
									<time
										dateTime={source.last_event_at}
										title={new Date(source.last_event_at).toLocaleString(
											'fr-FR',
										)}
									>
										{relativeAge(source.last_event_at)}
									</time>
								) : (
									'Jamais'
								)}
							</Td>
							<Td>
								<Sw
									aria-label={`Activer ${source.name}`}
									checked={source.enabled}
									disabled={busy || source.archived}
									onCheckedChange={(checked) => onToggle(source, checked)}
								/>
							</Td>
						</Tr>
					)
				})}
			</tbody>
		</Tbl>
	)
}
