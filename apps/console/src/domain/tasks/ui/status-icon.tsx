import type { DemoTaskStatus } from '#/domain/demo'
import { cn } from '#/lib/utils'

const TONE: Record<DemoTaskStatus, string> = {
	todo: 'text-signal-attention-ink',
	progress: 'text-primary',
	review: 'text-violet',
	done: 'text-signal-success',
	blocked: 'text-signal-danger',
	cancelled: 'text-faint',
	backlog: 'text-faint',
}

const RING = {
	cx: 8,
	cy: 8,
	r: 6,
	fill: 'none',
	strokeWidth: 1.6,
	className: 'stroke-current',
} as const

function Glyph({ status }: { status: DemoTaskStatus }) {
	switch (status) {
		case 'todo':
			return <circle {...RING} />
		case 'progress':
			return (
				<>
					<circle {...RING} className="stroke-border" />
					<path
						d="M8 2a6 6 0 0 1 6 6"
						fill="none"
						strokeWidth="1.8"
						strokeLinecap="round"
						className="origin-center animate-spin stroke-current [animation-duration:1.1s]"
					/>
				</>
			)
		case 'review':
			return (
				<>
					<circle {...RING} />
					<path d="M8 2a6 6 0 0 1 0 12z" className="fill-current" />
				</>
			)
		case 'done':
			return (
				<>
					<circle cx="8" cy="8" r="7" className="fill-current" />
					<path
						d="M5 8.2l2 2 4-4.4"
						fill="none"
						strokeWidth="1.6"
						strokeLinecap="round"
						strokeLinejoin="round"
						className="stroke-white"
					/>
				</>
			)
		case 'blocked':
			return (
				<>
					<circle {...RING} />
					<path
						d="M5 8h6"
						strokeWidth="1.8"
						strokeLinecap="round"
						className="stroke-current"
					/>
				</>
			)
		case 'cancelled':
			return (
				<>
					<circle {...RING} />
					<path
						d="M5.8 5.8l4.4 4.4M10.2 5.8l-4.4 4.4"
						strokeWidth="1.5"
						strokeLinecap="round"
						className="stroke-current"
					/>
				</>
			)
		case 'backlog':
			return <circle {...RING} strokeDasharray="2.4 2.2" />
	}
}

interface Props {
	status: DemoTaskStatus
	className?: string
}

export function StatusIcon({ status, className }: Props) {
	return (
		<svg
			viewBox="0 0 16 16"
			aria-hidden="true"
			className={cn('size-3.5 shrink-0', TONE[status], className)}
		>
			<Glyph status={status} />
		</svg>
	)
}
