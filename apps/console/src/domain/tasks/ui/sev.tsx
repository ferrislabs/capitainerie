import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

const TONE = {
	ok: 'border-signal-success/45 bg-signal-success/12 text-signal-success',
	info: 'border-border bg-secondary text-muted-foreground',
} as const

export function Sev({
	tone,
	children,
}: {
	tone: keyof typeof TONE
	children: ReactNode
}) {
	return (
		<span
			className={cn(
				'rounded-[6px] border px-2 py-px text-[11px] font-medium whitespace-nowrap',
				TONE[tone],
			)}
		>
			{children}
		</span>
	)
}
