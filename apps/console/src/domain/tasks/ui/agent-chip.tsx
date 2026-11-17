import { cn } from '#/lib/utils'

interface AvatarProps {
	hue: number
	size?: number
	label?: string
	className?: string
}

export function AgentAvatar({ hue, size = 20, label, className }: AvatarProps) {
	return (
		<span
			{...(label
				? { role: 'img', 'aria-label': label }
				: { 'aria-hidden': true })}
			className={cn(
				'inline-grid shrink-0 place-items-center rounded-full',
				className,
			)}
			style={{
				width: size,
				height: size,
				background: `linear-gradient(140deg,hsl(${hue} 70% 62%),hsl(${(hue + 70) % 360} 72% 56%))`,
			}}
		>
			<svg viewBox="0 0 16 16" className="size-[62%]" aria-hidden="true">
				<circle cx="5.5" cy="6.5" r="1.2" className="fill-zinc-900" />
				<circle cx="10.5" cy="6.5" r="1.2" className="fill-zinc-900" />
				<path
					d="M5 10c1.5 1.5 4.5 1.5 6 0"
					fill="none"
					strokeWidth="1.2"
					strokeLinecap="round"
					className="stroke-zinc-900"
				/>
			</svg>
		</span>
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
