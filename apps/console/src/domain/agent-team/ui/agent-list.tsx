import { Link } from '@tanstack/react-router'
import { buttonVariants } from '#/components/ui/button'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import { Live, ROW, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
import { cn } from '#/lib/utils'

interface Props {
	agents: ReadonlyArray<TeamAgent>
}

export function AgentList({ agents }: Props) {
	return (
		<div>
			{agents.map((agent) => (
				<Link
					key={agent.id}
					to="/agents/$agentId/{-$tab}"
					params={{ agentId: agent.id }}
					aria-label={`Configurer ${agent.name}`}
					className={ROW}
				>
					<AgentAvatar hue={agent.hue} size={28} />
					<Ttl
						title={agent.name}
						sub={`${agent.role} · ${agent.description}`}
					/>
					<span className="hidden text-xs text-muted-foreground md:inline">
						{agent.model}
					</span>
					{agent.status === 'running' ? (
						<Live />
					) : (
						<Sev tone="info">inactif</Sev>
					)}
					<span
						className={cn(
							buttonVariants({ variant: 'ghost', size: 'sm' }),
							'hidden md:inline-flex',
						)}
					>
						Configurer
					</span>
				</Link>
			))}
		</div>
	)
}
