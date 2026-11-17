import { cn } from '#/lib/utils'

interface AgentFaceProps {
	hue: number
	size?: number
	className?: string
}

export function AgentFace({ hue, size = 20, className }: AgentFaceProps) {
	return (
		<span
			aria-hidden="true"
			className={cn(
				'inline-grid shrink-0 place-items-center rounded-full',
				className,
			)}
			style={{
				width: size,
				height: size,
				background: `linear-gradient(140deg, hsl(${hue} 70% 62%), hsl(${(hue + 70) % 360} 72% 56%))`,
			}}
		>
			<svg viewBox="0 0 16 16" aria-hidden="true" className="size-[62%]">
				<circle cx="5.5" cy="6.5" r="1.2" className="fill-black/85" />
				<circle cx="10.5" cy="6.5" r="1.2" className="fill-black/85" />
				<path
					d="M5 10c1.5 1.5 4.5 1.5 6 0"
					fill="none"
					strokeWidth="1.2"
					strokeLinecap="round"
					className="stroke-black/85"
				/>
			</svg>
		</span>
	)
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
