import { Link } from '@tanstack/react-router'
import { Badge } from '#/components/ui/badge'
import { Switch } from '#/components/ui/switch'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { DemoAgent, DemoSource } from '#/domain/demo'
import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'
import { SourceTypeTile } from '#/domain/sources/ui/source-type-tile'

interface Props {
	sources: ReadonlyArray<DemoSource>
	agents: ReadonlyArray<DemoAgent>
	onToggle: (source: DemoSource, enabled: boolean) => void
}

export function SourcesTable({ sources, agents, onToggle }: Props) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Source</TableHead>
					<TableHead>Type</TableHead>
					<TableHead>Motifs</TableHead>
					<TableHead>Agents</TableHead>
					<TableHead className="text-right">24 h</TableHead>
					<TableHead>Actif</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{sources.map((source) => {
					const listening = source.agentIds
						.map((id) => agents.find((a) => a.id === id))
						.filter((a): a is DemoAgent => a !== undefined)
					return (
						<TableRow key={source.id}>
							<TableCell>
								<span className="flex items-center gap-2">
									<SourceTypeTile type={source.type} size={24} />
									<span className="min-w-0">
										<Link
											to="/sources/$sourceId/{-$tab}"
											params={{ sourceId: source.id }}
											className="font-medium text-foreground hover:underline"
										>
											{source.name}
										</Link>
										<span className="block text-xs text-muted-foreground">
											Dernier événement {source.last}
										</span>
									</span>
								</span>
							</TableCell>
							<TableCell>
								<Badge variant="muted">{source.type}</Badge>
							</TableCell>
							<TableCell className="max-w-60 font-mono break-words whitespace-normal">
								{source.patterns.join('  ') || '—'}
							</TableCell>
							<TableCell>
								{listening.length > 0 ? (
									<span className="flex -space-x-1">
										{listening.slice(0, 3).map((a) => (
											<AgentAvatar key={a.id} agent={a} />
										))}
									</span>
								) : (
									<Badge variant="attention">Aucun agent</Badge>
								)}
							</TableCell>
							<TableCell className="text-right figures">
								{source.eventsLast24h}
							</TableCell>
							<TableCell>
								<Switch
									aria-label={`Activer ${source.name}`}
									checked={source.enabled}
									onCheckedChange={(checked) => onToggle(source, checked)}
								/>
							</TableCell>
						</TableRow>
					)
				})}
			</TableBody>
		</Table>
	)
}
