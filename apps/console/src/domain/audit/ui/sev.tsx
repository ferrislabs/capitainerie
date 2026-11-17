import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

export type SevTone = 'ok' | 'crit' | 'warn' | 'info' | 'vio'

export const SEV_TONES: Record<SevTone, string> = {
	ok: 'border-signal-success/45 bg-signal-success/12 text-signal-success',
	crit: 'border-signal-danger/45 bg-signal-danger/12 text-signal-danger',
	warn: 'border-signal-attention-ink/45 bg-signal-attention-ink/12 text-signal-attention-ink',
	info: 'border-border bg-secondary text-muted-foreground',
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
				'rounded-md border px-2 py-px text-[11px] font-medium whitespace-nowrap',
				SEV_TONES[tone],
				className,
			)}
		>
			{children}
		</span>
	)
}
