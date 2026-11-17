import type { DemoAgent } from '#/domain/demo'

interface Props {
	agent: Pick<DemoAgent, 'name' | 'hue'>
	size?: number
	decorative?: boolean
}

export function AgentAvatar({ agent, size = 20, decorative = false }: Props) {
	const hue = agent.hue
	return (
		<span
			{...(decorative
				? { 'aria-hidden': true }
				: { role: 'img', 'aria-label': agent.name, title: agent.name })}
			className="inline-grid shrink-0 place-items-center rounded-full"
			style={{
				width: size,
				height: size,
				background: `linear-gradient(140deg, hsl(${hue} 70% 62%), hsl(${(hue + 70) % 360} 72% 56%))`,
			}}
		>
			<svg
				viewBox="0 0 16 16"
				className="size-[62%] text-neutral-900"
				aria-hidden="true"
			>
				<circle cx="5.5" cy="6.5" r="1.2" fill="currentColor" />
				<circle cx="10.5" cy="6.5" r="1.2" fill="currentColor" />
				<path
					d="M5 10c1.5 1.5 4.5 1.5 6 0"
					fill="none"
					stroke="currentColor"
					strokeWidth="1.2"
					strokeLinecap="round"
				/>
			</svg>
		</span>
	)
}
