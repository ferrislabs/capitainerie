import { useId } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	hue: number
	size?: number
	label?: string
	className?: string
}

const INK = '#1f2430'

function pincer(cx: number, cy: number, dir: number): string {
	const r = 3.6
	const gap = 26
	const rad = (deg: number) => (deg * Math.PI) / 180
	const x1 = cx + r * Math.cos(rad(dir + gap))
	const y1 = cy + r * Math.sin(rad(dir + gap))
	const x2 = cx + r * Math.cos(rad(dir - gap))
	const y2 = cy + r * Math.sin(rad(dir - gap))
	return `M${cx} ${cy}L${x1.toFixed(2)} ${y1.toFixed(2)}A${r} ${r} 0 1 1 ${x2.toFixed(2)} ${y2.toFixed(2)}Z`
}

const LEFT_PINCER = pincer(4.6, 12.4, -120)
const RIGHT_PINCER = pincer(27.4, 12.4, -60)

/** Petit crabe fixe, même palette que la mascotte animée, pour les listes et les puces. */
export function CrabAvatar({ hue, size = 20, label, className }: Props) {
	const id = useId()
	const shell = `${id}-shell`
	const limb = `${id}-limb`
	return (
		<span
			{...(label
				? { role: 'img', 'aria-label': label, title: label }
				: { 'aria-hidden': true })}
			className={cn('inline-grid shrink-0 place-items-center', className)}
			style={{ width: size, height: size }}
		>
			<svg
				viewBox="0 0 32 32"
				aria-hidden="true"
				className="size-full overflow-visible"
			>
				<defs>
					<linearGradient id={shell} x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stopColor={`hsl(${(hue + 200) % 360} 78% 76%)`} />
						<stop offset="1" stopColor={`hsl(${hue} 74% 66%)`} />
					</linearGradient>
					<linearGradient id={limb} x1="0" y1="0" x2="0" y2="1">
						<stop offset="0" stopColor={`hsl(${hue} 70% 70%)`} />
						<stop offset="1" stopColor={`hsl(${hue} 66% 60%)`} />
					</linearGradient>
				</defs>
				<g
					fill="none"
					stroke={`url(#${limb})`}
					strokeLinecap="round"
					strokeWidth="2.2"
				>
					<path d="M9 21.5 Q6.5 23.5 6 26" />
					<path d="M12 23.5 Q11 25.5 11 27.5" />
					<path d="M23 21.5 Q25.5 23.5 26 26" />
					<path d="M20 23.5 Q21 25.5 21 27.5" />
					<path d="M8 17 Q5.5 16 4.8 13" />
					<path d="M24 17 Q26.5 16 27.2 13" />
					<path d="M13 13 L12.4 8.6" strokeWidth="1.6" />
					<path d="M19 13 L19.6 8.6" strokeWidth="1.6" />
				</g>
				<path d={LEFT_PINCER} fill={`url(#${limb})`} />
				<path d={RIGHT_PINCER} fill={`url(#${limb})`} />
				<path
					d="M6.5 18.5 C6.5 12.8 10.6 10.8 16 10.8 C21.4 10.8 25.5 12.8 25.5 18.5 C25.5 21.6 22.3 23.2 16 23.2 C9.7 23.2 6.5 21.6 6.5 18.5 Z"
					fill={`url(#${shell})`}
				/>
				<circle cx="12.3" cy="7.4" r="2.5" fill="#fff" />
				<circle cx="19.7" cy="7.4" r="2.5" fill="#fff" />
				<circle cx="12.6" cy="7.6" r="1.15" fill={INK} />
				<circle cx="20" cy="7.6" r="1.15" fill={INK} />
				<path
					d="M14 18.2 Q16 19.9 18 18.2"
					fill="none"
					stroke={INK}
					strokeWidth="1.2"
					strokeLinecap="round"
				/>
			</svg>
		</span>
	)
}
