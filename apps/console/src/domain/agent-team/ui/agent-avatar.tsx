import { initialOf } from '#/domain/agent-team/agent-identity'

interface Props {
	name: string
	hue: number
	size?: number
}

export function AgentAvatar({ name, hue, size = 28 }: Props) {
	return (
		<span
			aria-hidden="true"
			className="inline-flex shrink-0 items-center justify-center rounded-full font-semibold text-white"
			style={{
				width: size,
				height: size,
				fontSize: Math.round(size * 0.42),
				backgroundColor: `hsl(${hue} 55% 42%)`,
			}}
		>
			{initialOf(name)}
		</span>
	)
}
