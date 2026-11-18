import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'

interface Props {
	agent: { name: string; hue: number } | undefined
}

export function AgentLabel({ agent }: Props) {
	if (!agent)
		return (
			<span className="text-[12px] text-muted-foreground">Agent inconnu</span>
		)
	return (
		<span className="flex min-w-0 items-center gap-1.5 text-[12px] text-muted-foreground">
			<AgentAvatar agent={agent} decorative />
			<span className="truncate">{agent.name}</span>
		</span>
	)
}
