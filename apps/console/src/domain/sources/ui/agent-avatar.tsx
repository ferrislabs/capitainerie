import { CrabAvatar } from '#/components/crab-avatar'

interface Props {
	agent: { name: string; hue: number }
	size?: number
	decorative?: boolean
}

export function AgentAvatar({ agent, size = 20, decorative = false }: Props) {
	return (
		<CrabAvatar
			hue={agent.hue}
			size={size}
			label={decorative ? undefined : agent.name}
		/>
	)
}
