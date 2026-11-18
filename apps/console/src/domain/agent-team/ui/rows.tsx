import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

export const ROW =
	'flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 text-left first:border-t-0 hover:bg-secondary'

/** Ligne de la fiche agent : plus haute, marges alignées sur les cartes. */
export const DETAIL_ROW =
	'flex min-h-14 w-full min-w-0 items-center gap-3.5 border-t px-6 py-3 text-left first:border-t-0 hover:bg-secondary/60'

export function Rows({ children }: { children: ReactNode }) {
	return <div>{children}</div>
}

interface TtlProps {
	title: ReactNode
	sub?: ReactNode
	titleMono?: boolean
	subMono?: boolean
}

export function Ttl({ title, sub, titleMono, subMono }: TtlProps) {
	return (
		<span className="min-w-0 flex-1 leading-[1.4]">
			<b
				className={cn(
					'block truncate font-medium',
					titleMono && 'font-mono text-[11.5px]',
				)}
			>
				{title}
			</b>
			{sub ? (
				<span
					className={cn(
						'block truncate text-[12px] text-muted-foreground',
						subMono && 'font-mono',
					)}
				>
					{sub}
				</span>
			) : null}
		</span>
	)
}

export function Tag({ children }: { children: ReactNode }) {
	return (
		<span className="whitespace-nowrap font-mono text-[11px] font-medium text-faint">
			{children}
		</span>
	)
}

export function Live() {
	return (
		<span className="inline-flex items-center gap-[5px] text-[11px] text-primary before:size-1.5 before:animate-pulse before:rounded-full before:bg-primary">
			en cours
		</span>
	)
}

export function Empty({
	icon,
	title,
	children,
	className,
}: {
	icon?: ReactNode
	title?: ReactNode
	children?: ReactNode
	className?: string
}) {
	return (
		<div
			className={cn('px-6 py-10 text-center text-muted-foreground', className)}
		>
			{icon}
			{title ? (
				<b className="mt-1.5 mb-0.5 block font-semibold text-foreground">
					{title}
				</b>
			) : null}
			{children}
		</div>
	)
}

export function ChipList({ children }: { children: ReactNode }) {
	return <ul className="m-0 flex list-none flex-wrap gap-2 p-0">{children}</ul>
}

export function Chip({
	children,
	title,
}: {
	children: string
	title?: string
}) {
	return (
		<li
			title={title}
			className="rounded-full border bg-secondary px-3 py-1 text-[13px] font-medium text-foreground"
		>
			{children}
		</li>
	)
}
