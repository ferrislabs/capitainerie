import { useEffect, useId, useRef } from 'react'
import { hueOf } from '#/domain/agent-team/agent-identity'
import { useMediaQuery } from '#/hooks/use-media-query'
import { cn } from '#/lib/utils'

export type MascotState = 'idle' | 'running' | 'sleep'

interface Props {
	name: string
	state?: MascotState
	className?: string
	interactive?: boolean
	label?: string
}

const REDUCED_MOTION = '(prefers-reduced-motion: reduce)'
const INK = '#1f2430'
const SHELL =
	'M40 108 C40 72 66 60 100 60 C134 60 160 72 160 108 C160 127 140 134 100 134 C60 134 40 127 40 108 Z'
const MOUTH_SLEEP = 'M94 110 Q100 112 106 110'
const MOUTH_HAPPY = 'M90 105 Q100 119 110 105 Z'
const MOUTH_CALM = 'M92 107 Q100 114 108 107'
const LEGS = [
	'M64 120 Q50 130 46 143',
	'M72 126 Q63 138 61 149',
	'M82 129 Q78 141 79 151',
	'M136 120 Q150 130 154 143',
	'M128 126 Q137 138 139 149',
	'M118 129 Q122 141 121 151',
]

const PARTS = [
	'body',
	'shell',
	'shadow',
	'stalkL',
	'stalkR',
	'eyeL',
	'eyeR',
	'whiteL',
	'whiteR',
	'pupL',
	'pupR',
	'glintL',
	'glintR',
	'closedL',
	'closedR',
	'mouth',
	'armL',
	'armR',
	'pinL',
	'pinR',
] as const

type Part = (typeof PARTS)[number]
type Parts = Record<Part, SVGElement>

interface Motion {
	target: { x: number; y: number }
	cur: { x: number; y: number }
	joy: number
	hover: boolean
	nextBlink: number
	blinkT: number
}

export function mascotPalette(name: string) {
	const awake = name.trim().length > 0
	const h = awake ? hueOf(name) : 0
	const sat = (s: number) => (awake ? s : 6)
	const top = (h + 200) % 360
	return {
		shellTop: `hsl(${top} ${sat(78)}% 76%)`,
		shellBottom: `hsl(${h} ${sat(74)}% 66%)`,
		limbTop: `hsl(${h} ${sat(70)}% 70%)`,
		limbBottom: `hsl(${h} ${sat(66)}% 60%)`,
		legs: `hsl(${h} ${sat(58)}% 60%)`,
	}
}

function claw(cx: number, cy: number, r: number, dir: number, gap: number) {
	const rad = (d: number) => (d * Math.PI) / 180
	const p1 = [
		cx + r * Math.cos(rad(dir + gap)),
		cy + r * Math.sin(rad(dir + gap)),
	]
	const p2 = [
		cx + r * Math.cos(rad(dir - gap)),
		cy + r * Math.sin(rad(dir - gap)),
	]
	return `M${cx} ${cy} L${p1[0].toFixed(2)} ${p1[1].toFixed(2)} A${r} ${r} 0 1 1 ${p2[0].toFixed(2)} ${p2[1].toFixed(2)} Z`
}

function collect(svg: SVGSVGElement): Parts | null {
	const parts = {} as Parts
	for (const key of PARTS) {
		const el = svg.querySelector<SVGElement>(`[data-part="${key}"]`)
		if (!el) return null
		parts[key] = el
	}
	return parts
}

function draw(
	parts: Parts,
	m: Motion,
	now: number,
	state: MascotState,
	hover: boolean,
	reduce: boolean,
) {
	const set = (el: SVGElement, attr: string, value: string | number) =>
		el.setAttribute(attr, String(value))
	const t = now / 1000
	const asleep = state === 'sleep'
	const running = state === 'running'
	const { cur, target } = m
	if (reduce) {
		cur.x = asleep ? 0 : target.x
		cur.y = asleep ? 0.3 : target.y
		m.joy = 0
	} else {
		const follow = asleep ? 0.03 : 0.12
		cur.x += ((asleep ? 0 : target.x) - cur.x) * follow
		cur.y += ((asleep ? 0.3 : target.y) - cur.y) * follow
		m.joy *= 0.95
	}
	const lean = reduce ? { x: 0, y: 0 } : cur

	const breathe = reduce
		? 0
		: Math.sin(t * (asleep ? 1.1 : 2)) * (asleep ? 0.03 : 0.016)
	const hop = reduce
		? 0
		: (running ? -Math.abs(Math.sin(t * 9)) * 4 : 0) -
			m.joy * 10 * Math.abs(Math.sin(t * 14))
	set(
		parts.body,
		'transform',
		`translate(${lean.x * 6} ${hop}) rotate(${lean.x * 6} 100 140)`,
	)
	set(
		parts.shell,
		'transform',
		`translate(0 ${134 * breathe}) scale(1 ${1 - breathe})`,
	)
	set(parts.shadow, 'rx', 54 + hop * 0.9)

	const ex = lean.x * 4
	const ey = lean.y * 2
	set(parts.stalkL, 'x2', 82 + ex)
	set(parts.stalkL, 'y2', 46 + ey)
	set(parts.stalkR, 'x2', 118 + ex)
	set(parts.stalkR, 'y2', 46 + ey)
	set(parts.eyeL, 'transform', `translate(${ex} ${ey})`)
	set(parts.eyeR, 'transform', `translate(${ex} ${ey})`)

	if (!reduce && !asleep && now > m.nextBlink && m.blinkT < 0) m.blinkT = now
	let lid = 1
	if (m.blinkT >= 0) {
		const p = (now - m.blinkT) / 170
		lid = p < 1 ? Math.abs(1 - 2 * p) : 1
		if (p >= 1) {
			m.blinkT = -1
			m.nextBlink = now + 2400 + Math.random() * 3600
		}
	}
	const open = asleep ? 0 : Math.max(0.1, lid)
	const visible = open > 0.15
	for (const [bx, white, pup, glint, closed] of [
		[82, parts.whiteL, parts.pupL, parts.glintL, parts.closedL],
		[118, parts.whiteR, parts.pupR, parts.glintR, parts.closedR],
	] as const) {
		set(white, 'ry', 10 * open)
		set(pup, 'cx', bx + cur.x * 4.4)
		set(pup, 'cy', 42 + cur.y * 4.4 * open)
		set(pup, 'ry', 4.6 * open)
		set(glint, 'cx', bx - 2 + cur.x * 4.4)
		set(glint, 'cy', 40 + cur.y * 4.4)
		set(white, 'opacity', visible ? '1' : '0')
		set(pup, 'opacity', visible ? '1' : '0')
		set(glint, 'opacity', open > 0.6 ? '1' : '0')
		set(closed, 'opacity', visible ? '0' : '1')
	}

	const happy = !asleep && (hover || m.joy > 0.2 || running)
	set(parts.mouth, 'd', asleep ? MOUTH_SLEEP : happy ? MOUTH_HAPPY : MOUTH_CALM)
	set(parts.mouth, 'fill', happy ? INK : 'none')

	const speed = running ? 10 : hover ? 6 : 1.6
	const amp = reduce ? 0 : running ? 14 : hover ? 11 : asleep ? 1.5 : 4
	const shake = m.joy * 22 * Math.sin(t * 20)
	set(
		parts.armL,
		'transform',
		`rotate(${-(Math.sin(t * speed) * amp + shake) - lean.y * 4} 58 102)`,
	)
	set(
		parts.armR,
		'transform',
		`rotate(${Math.sin(t * speed + 1.3) * amp + shake + lean.y * 4} 142 102)`,
	)

	const pinch = reduce
		? 0.4
		: asleep
			? 0.05
			: (Math.sin(t * (running ? 12 : hover ? 7 : 2.6)) + 1) / 2
	const gap = (asleep ? 6 : 12) + pinch * (running ? 34 : hover ? 28 : 16)
	set(parts.pinL, 'd', claw(32, 74, 15, -120, gap))
	set(parts.pinR, 'd', claw(168, 74, 15, -60, gap))
}

export function Mascot({
	name,
	state = 'idle',
	className = 'h-[104px] w-[122px]',
	interactive = true,
	label,
}: Props) {
	const uid = `mascot-${useId().replace(/[^a-zA-Z0-9_-]/g, '')}`
	const ids = {
		shell: `${uid}-shell`,
		limb: `${uid}-limb`,
		shine: `${uid}-shine`,
	}
	const reduce = useMediaQuery(REDUCED_MOTION)
	const svgRef = useRef<SVGSVGElement>(null)
	const stateRef = useRef(state)
	const interactiveRef = useRef(interactive)
	const redrawRef = useRef<(() => void) | null>(null)
	const motion = useRef<Motion>({
		target: { x: 0, y: 0 },
		cur: { x: 0, y: 0 },
		joy: 0,
		hover: false,
		nextBlink: 0,
		blinkT: -1,
	})
	stateRef.current = state
	interactiveRef.current = interactive

	useEffect(() => {
		const svg = svgRef.current
		const parts = svg ? collect(svg) : null
		if (!svg || !parts) return
		const m = motion.current
		if (m.nextBlink === 0) m.nextBlink = performance.now() + 2500
		const hovered = () => interactiveRef.current && m.hover
		let raf: number | null = null
		let onScreen = true

		const frame = (now: number) => {
			draw(parts, m, now, stateRef.current, hovered(), false)
			raf = requestAnimationFrame(frame)
		}
		const sync = () => {
			const run = !reduce && onScreen && !document.hidden
			if (run && raf === null) raf = requestAnimationFrame(frame)
			if (!run && raf !== null) {
				cancelAnimationFrame(raf)
				raf = null
			}
		}
		const still = () =>
			draw(parts, m, performance.now(), stateRef.current, hovered(), true)
		redrawRef.current = reduce ? still : null

		const onMove = (event: PointerEvent) => {
			if (!onScreen) return
			const r = svg.getBoundingClientRect()
			const dx = event.clientX - (r.left + r.width / 2)
			const dy = event.clientY - (r.top + r.height * 0.28)
			const d = Math.hypot(dx, dy) || 1
			const k = Math.min(1, d / 240)
			m.target.x = (dx / d) * k
			m.target.y = (dy / d) * k
			if (reduce) still()
		}
		const hover = (on: boolean) => () => {
			m.hover = on
			if (reduce) still()
		}
		const enter = hover(true)
		const leave = hover(false)
		const poke = () => {
			if (interactiveRef.current) m.joy = 1
		}
		svg.addEventListener('pointerenter', enter)
		svg.addEventListener('pointerleave', leave)
		svg.addEventListener('click', poke)
		window.addEventListener('pointermove', onMove, { passive: true })
		document.addEventListener('visibilitychange', sync)
		const observer =
			typeof IntersectionObserver === 'function'
				? new IntersectionObserver((entries) => {
						onScreen = entries.some((entry) => entry.isIntersecting)
						sync()
					})
				: null
		observer?.observe(svg)
		if (reduce) still()
		sync()

		return () => {
			svg.removeEventListener('pointerenter', enter)
			svg.removeEventListener('pointerleave', leave)
			svg.removeEventListener('click', poke)
			window.removeEventListener('pointermove', onMove)
			document.removeEventListener('visibilitychange', sync)
			observer?.disconnect()
			if (raf !== null) cancelAnimationFrame(raf)
			redrawRef.current = null
		}
	}, [reduce])

	useEffect(() => {
		if (state || interactive) redrawRef.current?.()
	}, [state, interactive])

	const palette = mascotPalette(name)
	const asleep = state === 'sleep'
	const pin = asleep ? 6 + 0.05 * 16 : 18.4
	const zzz = cn(
		'fill-muted-foreground font-sans text-[15px] font-bold transition-opacity duration-[400ms]',
		asleep
			? 'animate-mascot-zz opacity-100 motion-reduce:animate-none'
			: 'opacity-0',
	)

	return (
		<svg
			ref={svgRef}
			viewBox="0 0 200 170"
			className={cn(
				'overflow-visible',
				interactive && 'cursor-pointer',
				className,
			)}
			data-state={state}
			role={label ? 'img' : undefined}
			aria-label={label}
			aria-hidden={label ? undefined : true}
		>
			{label ? <title>{label}</title> : null}
			<defs>
				<linearGradient id={ids.shell} x1="0" y1="0" x2="0" y2="1">
					<stop offset="0" stopColor={palette.shellTop} />
					<stop offset="1" stopColor={palette.shellBottom} />
				</linearGradient>
				<linearGradient id={ids.limb} x1="0" y1="0" x2="0" y2="1">
					<stop offset="0" stopColor={palette.limbTop} />
					<stop offset="1" stopColor={palette.limbBottom} />
				</linearGradient>
				<radialGradient id={ids.shine} cx="0.35" cy="0.2" r="0.65">
					<stop offset="0" stopColor="#fff" stopOpacity=".5" />
					<stop offset="1" stopColor="#fff" stopOpacity="0" />
				</radialGradient>
			</defs>

			<ellipse
				data-part="shadow"
				cx="100"
				cy="157"
				rx="54"
				ry="7"
				fill="currentColor"
				opacity=".16"
			/>

			<g data-part="body">
				<g
					stroke={palette.legs}
					strokeLinecap="round"
					strokeWidth="7"
					fill="none"
				>
					{LEGS.map((d) => (
						<path key={d} d={d} />
					))}
				</g>

				<g data-part="armL">
					<path
						d="M58 102 Q42 98 34 80"
						stroke={`url(#${ids.limb})`}
						strokeWidth="8"
						strokeLinecap="round"
						fill="none"
					/>
					<path
						data-part="pinL"
						d={claw(32, 74, 15, -120, pin)}
						fill={`url(#${ids.limb})`}
					/>
				</g>
				<g data-part="armR">
					<path
						d="M142 102 Q158 98 166 80"
						stroke={`url(#${ids.limb})`}
						strokeWidth="8"
						strokeLinecap="round"
						fill="none"
					/>
					<path
						data-part="pinR"
						d={claw(168, 74, 15, -60, pin)}
						fill={`url(#${ids.limb})`}
					/>
				</g>

				<g stroke={`url(#${ids.limb})`} strokeWidth="5" strokeLinecap="round">
					<line data-part="stalkL" x1="85" y1="72" x2="82" y2="46" />
					<line data-part="stalkR" x1="115" y1="72" x2="118" y2="46" />
				</g>

				<g data-part="shell">
					<path d={SHELL} fill={`url(#${ids.shell})`} />
					<path d={SHELL} fill={`url(#${ids.shine})`} />
					<ellipse
						cx="72"
						cy="109"
						rx="7"
						ry="4"
						fill="#ff6f86"
						opacity=".32"
					/>
					<ellipse
						cx="128"
						cy="109"
						rx="7"
						ry="4"
						fill="#ff6f86"
						opacity=".32"
					/>
					<path
						data-part="mouth"
						d={
							asleep
								? MOUTH_SLEEP
								: state === 'running'
									? MOUTH_HAPPY
									: MOUTH_CALM
						}
						fill={state === 'running' ? INK : 'none'}
						stroke={INK}
						strokeWidth="3"
						strokeLinecap="round"
						strokeLinejoin="round"
					/>
				</g>

				{(
					[
						['L', 82],
						['R', 118],
					] as const
				).map(([side, x]) => (
					<g key={side} data-part={`eye${side}`}>
						<ellipse
							data-part={`white${side}`}
							cx={x}
							cy="42"
							rx="10"
							ry={asleep ? 0 : 10}
							fill="#fff"
							opacity={asleep ? 0 : 1}
						/>
						<ellipse
							data-part={`pup${side}`}
							cx={x}
							cy="42"
							rx="4.6"
							ry={asleep ? 0 : 4.6}
							fill={INK}
							opacity={asleep ? 0 : 1}
						/>
						<circle
							data-part={`glint${side}`}
							cx={x - 2}
							cy="40"
							r="1.5"
							fill="#fff"
							opacity={asleep ? 0 : 1}
						/>
						<path
							data-part={`closed${side}`}
							d={`M${x - 8} 43 Q${x} 48 ${x + 8} 43`}
							stroke={INK}
							strokeWidth="3"
							strokeLinecap="round"
							fill="none"
							opacity={asleep ? 1 : 0}
						/>
					</g>
				))}
			</g>
			<text className={zzz} x="138" y="30">
				z
			</text>
			<text className={cn(zzz, '[animation-delay:1.2s]')} x="146" y="22">
				z
			</text>
		</svg>
	)
}
