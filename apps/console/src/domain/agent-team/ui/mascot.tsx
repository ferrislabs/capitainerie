import { useId } from 'react'
import { hueOf } from '#/domain/agent-team/agent-identity'

interface Props {
	name: string
	className?: string
}

export function Mascot({ name, className = 'h-[104px] w-28' }: Props) {
	const gradient = useId()
	const awake = name.trim().length > 0
	const hue = awake ? hueOf(name) : 0
	const sat = awake ? 70 : 6
	return (
		<svg viewBox="0 0 140 130" aria-hidden="true" className={className}>
			<defs>
				<linearGradient id={gradient} x1="0" y1="0" x2="0" y2="1">
					<stop
						offset="0"
						stopColor={`hsl(${hue} ${sat}% ${awake ? 64 : 62}%)`}
					/>
					<stop
						offset="1"
						stopColor={`hsl(${(hue + 60) % 360} ${sat}% ${awake ? 52 : 32}%)`}
					/>
				</linearGradient>
			</defs>
			<ellipse cx="70" cy="118" rx="40" ry="5" className="fill-border" />
			<path
				d="M26 108V62C26 34 46 16 70 16s44 18 44 46v46z"
				fill={`url(#${gradient})`}
			/>
			{awake ? (
				<>
					<circle cx="56" cy="64" r="4" fill="#1b1d22" />
					<circle cx="84" cy="64" r="4" fill="#1b1d22" />
					<path
						d="M58 78c6 6 18 6 24 0"
						fill="none"
						stroke="#1b1d22"
						strokeWidth="3"
						strokeLinecap="round"
					/>
				</>
			) : (
				<>
					<path
						d="M50 66q6 5 12 0M78 66q6 5 12 0M66 78h8"
						fill="none"
						stroke="#1b1d22"
						strokeWidth="2.5"
						strokeLinecap="round"
					/>
					<text
						x="104"
						y="30"
						fontSize="14"
						fontWeight="700"
						className="fill-signal-attention-ink"
					>
						z
					</text>
					<text
						x="114"
						y="18"
						fontSize="10"
						fontWeight="700"
						className="fill-signal-attention-ink"
					>
						z
					</text>
				</>
			)}
		</svg>
	)
}
