import { DEMO_AGENTS, shortName } from '#/domain/demo'
import { AgentAvatar as GradientAvatar } from '#/domain/sources/ui/agent-avatar'

interface Props {
	name: string
	size?: number
}

const FALLBACK_HUE = DEMO_AGENTS[1]?.hue ?? 250

export function AgentAvatar({ name, size = 18 }: Props) {
	const hue = DEMO_AGENTS.find((a) => a.name === name)?.hue ?? FALLBACK_HUE
	return <GradientAvatar agent={{ name, hue }} size={size} decorative />
}

export function AgentLabel({ name }: { name: string }) {
	return (
		<span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-[12px] text-ellipsis whitespace-nowrap text-muted-foreground">
			<AgentAvatar name={name} />
			{shortName(name)}
		</span>
	)
}
