interface Props {
	hue: number
	size?: number
}

export function AgentAvatar({ hue, size = 28 }: Props) {
	return (
		<span
			aria-hidden="true"
			className="inline-grid shrink-0 place-items-center rounded-full"
			style={{
				width: size,
				height: size,
				background: `linear-gradient(140deg,hsl(${hue} 70% 62%),hsl(${(hue + 70) % 360} 72% 56%))`,
			}}
		>
			<svg viewBox="0 0 16 16" aria-hidden="true" className="size-[62%]">
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
