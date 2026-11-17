import { Link } from '@tanstack/react-router'
import { Badge } from '#/components/ui/badge'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import type { DemoAgent } from '#/domain/demo'

export interface OrgEntry {
	agent: DemoAgent
	depth: number
	subCount: number
	canDelegate: boolean
}

interface Props {
	entries: ReadonlyArray<OrgEntry>
}

export function OrgTreeView({ entries }: Props) {
	return (
		<ul className="divide-y">
			{entries.map(({ agent, depth, subCount, canDelegate }) => (
				<li key={agent.id}>
					<Link
						to="/agents/equipe/$agentId/{-$tab}"
						params={{ agentId: agent.id }}
						className="flex items-center gap-2.5 py-2 pr-3 hover:bg-accent"
						style={{ paddingLeft: 12 + depth * 22 }}
					>
						{depth > 0 ? (
							<span aria-hidden="true" className="text-muted-foreground">
								↳
							</span>
						) : null}
						<AgentAvatar name={agent.name} hue={agent.hue} size={24} />
						<span className="min-w-0 flex-1">
							<span className="block truncate font-medium">{agent.name}</span>
							<span className="block truncate text-xs text-muted-foreground">
								{agent.role}
								{canDelegate ? ' · peut déléguer' : ''}
							</span>
						</span>
						{subCount > 0 ? (
							<Badge variant="outline">
								{subCount} {subCount > 1 ? 'sous-agents' : 'sous-agent'}
							</Badge>
						) : null}
						{agent.status === 'running' ? (
							<Badge variant="running">en cours</Badge>
						) : null}
					</Link>
				</li>
			))}
		</ul>
	)
}
