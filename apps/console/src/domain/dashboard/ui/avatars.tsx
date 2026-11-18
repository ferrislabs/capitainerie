import { CrabAvatar } from '#/components/crab-avatar'
import { cn } from '#/lib/utils'

interface AgentFaceProps {
	hue: number
	size?: number
	className?: string
}

export function AgentFace({ hue, size = 20, className }: AgentFaceProps) {
	return <CrabAvatar hue={hue} size={size} className={className} />
}

interface HumanAvatarProps {
	initials: string
	size?: number
	className?: string
}

export function HumanAvatar({
	initials,
	size = 20,
	className,
}: HumanAvatarProps) {
	return (
		<span
			aria-hidden="true"
			className={cn(
				'inline-grid shrink-0 place-items-center rounded-full bg-secondary text-[9px] font-semibold text-muted-foreground',
				className,
			)}
			style={{ width: size, height: size }}
		>
			{initials}
		</span>
	)
}
