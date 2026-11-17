import type { ComponentProps, ReactNode } from 'react'
import { Switch } from '#/components/ui/switch'
import { cn } from '#/lib/utils'

export const CTL =
	'w-full min-w-0 rounded-md border border-border bg-background px-[9px] py-[5px] text-foreground outline-none placeholder:text-faint focus:border-primary'

export const MUTED_SMALL = 'text-[12px] text-muted-foreground'

const SEV_TONES = {
	crit: 'border-signal-danger/45 bg-signal-danger/12 text-signal-danger',
	warn: 'border-signal-attention-ink/45 bg-signal-attention-ink/12 text-signal-attention-ink',
	info: 'border-border bg-secondary text-muted-foreground',
	ok: 'border-signal-success/45 bg-signal-success/12 text-signal-success',
	vio: 'border-violet/45 bg-violet/12 text-violet',
} as const

export type SevTone = keyof typeof SEV_TONES

export function Sev({
	tone,
	className,
	...props
}: ComponentProps<'span'> & { tone: SevTone }) {
	return (
		<span
			className={cn(
				'rounded-md border px-2 py-px text-[11px] font-medium whitespace-nowrap',
				SEV_TONES[tone],
				className,
			)}
			{...props}
		/>
	)
}

export function Tbl({ className, ...props }: ComponentProps<'table'>) {
	return (
		<div className="overflow-x-auto">
			<table className={cn('w-full border-collapse', className)} {...props} />
		</div>
	)
}

export function Th({ className, ...props }: ComponentProps<'th'>) {
	return (
		<th
			className={cn(
				'border-b px-3 py-[7px] text-left font-mono text-[10.5px] [line-height:normal] font-medium tracking-[0.06em] whitespace-nowrap text-faint uppercase',
				className,
			)}
			{...props}
		/>
	)
}

export function Td({ className, ...props }: ComponentProps<'td'>) {
	return (
		<td
			className={cn(
				'border-t px-3 py-[7px] align-middle text-base [line-height:normal]',
				className,
			)}
			{...props}
		/>
	)
}

export function Tr({ className, ...props }: ComponentProps<'tr'>) {
	return (
		<tr
			className={cn(
				'hover:bg-secondary [tbody>&:first-child>td]:border-t-0',
				className,
			)}
			{...props}
		/>
	)
}

export function Sw({ className, ...props }: ComponentProps<typeof Switch>) {
	return (
		<Switch
			className={cn(
				'h-4 w-7 border-0 bg-border p-0.5 data-[state=checked]:bg-primary [&>span]:size-3 [&>span]:bg-foreground [&>span]:data-[state=checked]:translate-x-3 [&>span]:data-[state=checked]:bg-white',
				className,
			)}
			{...props}
		/>
	)
}

export function Fr({
	label,
	hint,
	children,
	className,
}: {
	label: ReactNode
	hint?: ReactNode
	children: ReactNode
	className?: string
}) {
	return (
		<div
			className={cn(
				'grid min-w-0 grid-cols-[180px_minmax(0,1fr)] items-center gap-2.5 border-t px-3 py-2 first:border-t-0 max-[800px]:grid-cols-1 max-[800px]:gap-1',
				className,
			)}
		>
			<span className="text-muted-foreground">
				{label}
				{hint ? (
					<div className="mt-0.5 text-[11.5px] text-faint">{hint}</div>
				) : null}
			</span>
			<div className="min-w-0 [&_input]:max-w-[440px] [&_select]:max-w-[440px] [&_textarea]:max-w-[440px]">
				{children}
			</div>
		</div>
	)
}

export function Kv({
	label,
	children,
}: {
	label: ReactNode
	children: ReactNode
}) {
	return (
		<div className="grid min-w-0 grid-cols-[180px_minmax(0,1fr)] items-center gap-2.5 border-t px-3 py-2 first:border-t-0">
			<span className="text-muted-foreground">{label}</span>
			<span className="min-w-0 [overflow-wrap:anywhere]">{children}</span>
		</div>
	)
}

export function Flow({
	steps,
}: {
	steps: ReadonlyArray<{ title: string; body: ReactNode }>
}) {
	return (
		<ol className="m-0 flex list-none flex-wrap items-stretch gap-1.5 p-3">
			{steps.map((step, i) => (
				<li key={step.title} className="contents">
					<div className="min-w-35 flex-1 rounded-lg border bg-background px-2.5 py-2">
						<b className="mb-0.5 block font-semibold">{step.title}</b>
						<span className="text-[12px] text-muted-foreground [overflow-wrap:anywhere]">
							{step.body}
						</span>
					</div>
					{i < steps.length - 1 ? (
						<i aria-hidden="true" className="self-center text-faint not-italic">
							→
						</i>
					) : null}
				</li>
			))}
		</ol>
	)
}

export function AddRow({ className, ...props }: ComponentProps<'div'>) {
	return <div className={cn('flex gap-1.5 px-3 pb-3', className)} {...props} />
}

export function Empty({
	title,
	children,
}: {
	title: string
	children?: ReactNode
}) {
	return (
		<div className="px-4 py-[26px] text-center text-muted-foreground">
			<b className="mx-0 mt-1.5 mb-0.5 block font-semibold text-foreground">
				{title}
			</b>
			{children}
		</div>
	)
}
