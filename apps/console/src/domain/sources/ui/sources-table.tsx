import { Link } from '@tanstack/react-router'
import type { DemoAgent, DemoSource } from '#/domain/demo'
import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'
import { Sev, Sw, Tbl, Td, Th, Tr } from '#/domain/sources/ui/mockup'
import { SourceTypeTile } from '#/domain/sources/ui/source-type-tile'

interface Props {
	sources: ReadonlyArray<DemoSource>
	agents: ReadonlyArray<DemoAgent>
	onToggle: (source: DemoSource, enabled: boolean) => void
}

const AVS =
	'flex min-w-0 items-center gap-1.5 overflow-hidden text-[12px] leading-[normal] text-ellipsis whitespace-nowrap text-muted-foreground'

export function SourcesTable({ sources, agents, onToggle }: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Source</Th>
					<Th>Type</Th>
					<Th>Motifs</Th>
					<Th>Agents</Th>
					<Th>24 h</Th>
					<Th>Actif</Th>
				</tr>
			</thead>
			<tbody>
				{sources.map((source) => {
					const listening = source.agentIds
						.map((id) => agents.find((a) => a.id === id))
						.filter((a): a is DemoAgent => a !== undefined)
					return (
						<Tr key={source.id}>
							<Td>
								<span className={AVS}>
									<SourceTypeTile type={source.type} size={24} />
									<span>
										<Link
											to="/sources/$sourceId/{-$tab}"
											params={{ sourceId: source.id }}
											className="font-medium text-foreground"
										>
											{source.name}
										</Link>
										<div className="text-[12px] text-muted-foreground">
											Dernier événement {source.last}
										</div>
									</span>
								</span>
							</Td>
							<Td>
								<span className="font-mono text-[11px] font-medium whitespace-nowrap text-faint">
									{source.type}
								</span>
							</Td>
							<Td className="max-w-60 font-mono text-[11.5px] [overflow-wrap:anywhere]">
								{source.patterns.join('  ') || '—'}
							</Td>
							<Td>
								{listening.length > 0 ? (
									<span className={AVS}>
										{listening.slice(0, 3).map((a) => (
											<AgentAvatar key={a.id} agent={a} />
										))}
									</span>
								) : (
									<Sev tone="warn">Aucun agent</Sev>
								)}
							</Td>
							<Td className="tabular-nums">{source.eventsLast24h}</Td>
							<Td>
								<Sw
									aria-label={`Activer ${source.name}`}
									checked={source.enabled}
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
