import { Link } from '@tanstack/react-router'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import { Live, ROW, Tag, Ttl } from '#/domain/agent-team/ui/rows'
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
		<div>
			{entries.map(({ agent, depth, subCount, canDelegate }) => (
				<Link
					key={agent.id}
					to="/agents/$agentId/{-$tab}"
					params={{ agentId: agent.id }}
					className={ROW}
					style={{ paddingLeft: 12 + depth * 22 }}
				>
					{depth > 0 ? (
						<span aria-hidden="true" className="text-faint">
							↳
						</span>
					) : null}
					<AgentAvatar hue={agent.hue} size={24} />
					<Ttl
						title={agent.name}
						sub={`${agent.role}${canDelegate ? ' · peut déléguer' : ''}`}
					/>
					{subCount > 0 ? (
						<Tag>
							{subCount} {subCount > 1 ? 'sous-agents' : 'sous-agent'}
						</Tag>
					) : null}
					{agent.status === 'running' ? <Live /> : null}
				</Link>
			))}
		</div>
	)
}
