import type { DemoAgent } from '#/domain/demo'

interface Props {
	agent: Pick<DemoAgent, 'name' | 'hue'>
	size?: number
}

export function AgentAvatar({ agent, size = 20 }: Props) {
	return (
		<span
			role="img"
			aria-label={agent.name}
			title={agent.name}
			className="inline-flex shrink-0 items-center justify-center rounded-full text-[10px] font-semibold text-white"
			style={{
				width: size,
				height: size,
				background: `oklch(0.55 0.12 ${agent.hue})`,
			}}
		>
			{agent.name.slice(0, 1).toUpperCase()}
		</span>
	)
}
