import { DEMO_AGENTS } from '#/domain/demo'

interface AgentAvatarProps {
	name: string
	size?: number
}

interface HumanAvatarProps {
	initials: string
	size?: number
}

const FALLBACK_HUE = 250

const hueOf = (name: string): number =>
	DEMO_AGENTS.find((agent) => agent.name === name)?.hue ?? FALLBACK_HUE

export function AgentAvatar({ name, size = 20 }: AgentAvatarProps) {
	const hue = hueOf(name)
	return (
		<span
			aria-hidden="true"
			className="inline-grid shrink-0 place-items-center rounded-full"
			style={{
				width: size,
				height: size,
				background: `linear-gradient(140deg, hsl(${hue} 70% 62%), hsl(${(hue + 70) % 360} 72% 56%))`,
			}}
		>
			<svg viewBox="0 0 16 16" className="size-[62%]" aria-hidden="true">
				<circle cx="5.5" cy="6.5" r="1.2" fill="#1b1d22" />
				<circle cx="10.5" cy="6.5" r="1.2" fill="#1b1d22" />
				<path
					d="M5 10c1.5 1.5 4.5 1.5 6 0"
					fill="none"
					stroke="#1b1d22"
					strokeWidth="1.2"
					strokeLinecap="round"
				/>
			</svg>
		</span>
	)
}

export function HumanAvatar({ initials, size = 20 }: HumanAvatarProps) {
	return (
		<span
			aria-hidden="true"
			className="inline-grid shrink-0 place-items-center rounded-full bg-secondary text-[9px] leading-none font-semibold text-muted-foreground"
			style={{ width: size, height: size }}
		>
			{initials}
		</span>
	)
}
