import { CrabAvatar } from '#/components/crab-avatar'

interface Props {
	hue: number
	size?: number
}

export function AgentAvatar({ hue, size = 28 }: Props) {
	return <CrabAvatar hue={hue} size={size} />
}
