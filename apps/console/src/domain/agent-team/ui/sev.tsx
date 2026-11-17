import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

export type SevTone = 'crit' | 'warn' | 'info' | 'ok' | 'vio'

const TONES: Record<SevTone, string> = {
	crit: 'border-signal-danger/45 bg-signal-danger/12 text-signal-danger',
	warn: 'border-signal-attention/45 bg-signal-attention/12 text-signal-attention-ink',
	info: 'border-border bg-secondary text-muted-foreground',
	ok: 'border-signal-success/45 bg-signal-success/12 text-signal-success',
	vio: 'border-violet/45 bg-violet/12 text-violet',
}

interface Props {
	tone: SevTone
	children: ReactNode
	className?: string
}

export function Sev({ tone, children, className }: Props) {
	return (
		<span
			className={cn(
				'whitespace-nowrap rounded-md border px-2 py-px leading-[1.4] text-[11px] font-medium',
				TONES[tone],
				className,
			)}
		>
			{children}
		</span>
	)
}
