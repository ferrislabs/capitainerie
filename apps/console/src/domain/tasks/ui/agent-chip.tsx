import { CrabAvatar } from '#/components/crab-avatar'
import { cn } from '#/lib/utils'

interface AvatarProps {
	hue: number
	size?: number
	label?: string
	className?: string
}

export function AgentAvatar({ hue, size = 20, label, className }: AvatarProps) {
	return (
		<CrabAvatar hue={hue} size={size} label={label} className={className} />
	)
}

export function HumanAvatar({
	initials,
	size = 20,
}: {
	initials: string
	size?: number
}) {
	return (
		<span
			aria-hidden="true"
			className="inline-grid shrink-0 place-items-center rounded-full bg-secondary text-[9px] font-semibold text-muted-foreground"
			style={{ width: size, height: size }}
		>
			{initials}
		</span>
	)
}

interface Props {
	name: string
	hue: number
	size?: number
	showName?: boolean
	className?: string
}

export function AgentChip({
	name,
	hue,
	size = 18,
	showName = true,
	className,
}: Props) {
	return (
		<span
			className={cn(
				'flex min-w-0 items-center gap-1.5 overflow-hidden text-xs text-ellipsis whitespace-nowrap text-muted-foreground',
				className,
			)}
		>
			<AgentAvatar hue={hue} size={size} label={showName ? undefined : name} />
			{showName ? name : null}
		</span>
	)
}
