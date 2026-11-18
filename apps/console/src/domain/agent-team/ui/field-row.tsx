import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	label: string
	hint?: string
	children: ReactNode
}

export const CTL =
	'w-full min-w-0 rounded-md border bg-background px-3 py-[7px] text-foreground focus:border-primary focus:outline-0'

export function FieldRow({ label, hint, children }: Props) {
	return (
		<div className="grid min-w-0 gap-x-6 gap-y-2 border-t px-6 py-4 first:border-t-0 sm:grid-cols-[220px_minmax(0,1fr)] sm:items-center [&_select]:max-w-[460px] [&_input:not([type=checkbox])]:max-w-[460px]">
			<span className="text-muted-foreground">
				{label}
				{hint ? (
					<div className="mt-1 text-[12.5px] leading-snug text-faint">
						{hint}
					</div>
				) : null}
			</span>
			<div className="min-w-0 [overflow-wrap:anywhere]">{children}</div>
		</div>
	)
}

interface InfoProps {
	label: string
	mono?: boolean
	children: ReactNode
}

/** Ligne de lecture : libellé discret à gauche, valeur à droite. */
export function InfoRow({ label, mono = false, children }: InfoProps) {
	return (
		<div className="flex min-w-0 items-baseline justify-between gap-6 py-3 first:pt-0 last:pb-0">
			<dt className="shrink-0 text-muted-foreground">{label}</dt>
			<dd
				className={cn(
					'm-0 min-w-0 text-right text-foreground [overflow-wrap:anywhere]',
					mono && 'font-mono text-[13.5px]',
				)}
			>
				{children}
			</dd>
		</div>
	)
}

export function InfoList({ children }: { children: ReactNode }) {
	return <dl className="m-0 flex flex-col">{children}</dl>
}

export function Note({ children }: { children: ReactNode }) {
	return (
		<p className="m-0 border-t px-6 py-4 text-muted-foreground">{children}</p>
	)
}
