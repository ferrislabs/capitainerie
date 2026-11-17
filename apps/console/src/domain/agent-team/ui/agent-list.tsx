import { Link } from '@tanstack/react-router'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import type { DemoAgent } from '#/domain/demo'

interface Props {
	agents: ReadonlyArray<DemoAgent>
}

export function AgentList({ agents }: Props) {
	return (
		<ul className="divide-y">
			{agents.map((agent) => (
				<li key={agent.id} className="flex items-center gap-3 px-3 py-2.5">
					<AgentAvatar name={agent.name} hue={agent.hue} />
					<div className="min-w-0 flex-1">
						<div className="truncate font-medium">{agent.name}</div>
						<div className="truncate text-xs text-muted-foreground">
							{agent.role} · {agent.description}
						</div>
					</div>
					<span className="hidden text-xs text-muted-foreground md:inline">
						{agent.model}
					</span>
					{agent.status === 'running' ? (
						<Badge variant="running">en cours</Badge>
					) : (
						<Badge variant="neutral">inactif</Badge>
					)}
					<Button asChild variant="outline" size="sm">
						<Link
							to="/agents/$agentId/{-$tab}"
							params={{ agentId: agent.id }}
							aria-label={`Configurer ${agent.name}`}
						>
							Configurer
						</Link>
					</Button>
				</li>
			))}
		</ul>
	)
}
