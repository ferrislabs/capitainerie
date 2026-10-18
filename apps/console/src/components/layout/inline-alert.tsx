import { CircleAlert, CircleCheck, Info } from 'lucide-react'
import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

type Tone = 'danger' | 'success' | 'info'

const TONES: Record<Tone, { className: string; icon: typeof Info }> = {
	danger: {
		className:
			'border-signal-danger/35 bg-signal-danger-soft text-signal-danger',
		icon: CircleAlert,
	},
	success: {
		className:
			'border-signal-success/35 bg-signal-success-soft text-signal-success',
		icon: CircleCheck,
	},
	info: {
		className:
			'border-signal-running/30 bg-signal-running-soft text-signal-running',
		icon: Info,
	},
}

interface Props {
	tone: Tone
	children: ReactNode
	className?: string
}

/** Message en ligne. `danger` est annoncé (role="alert"), les autres sont polis. */
export function InlineAlert({ tone, children, className }: Props) {
	const { className: toneClass, icon: Icon } = TONES[tone]
	const content = (
		<>
			<Icon aria-hidden="true" className="mt-px size-4 shrink-0" />
			<span className="min-w-0 text-[13px] font-medium">{children}</span>
		</>
	)
	const classes = cn(
		'flex items-start gap-2 rounded-md border px-3 py-2',
		toneClass,
		className,
	)

	if (tone === 'danger') {
		return (
			<p role="alert" className={classes}>
				{content}
			</p>
		)
	}
	return <output className={classes}>{content}</output>
}
