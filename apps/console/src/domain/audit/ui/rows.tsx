import type { LucideIcon } from 'lucide-react'
import type { ComponentProps, ReactNode } from 'react'
import { cn } from '#/lib/utils'

export function Panel({ className, ...props }: ComponentProps<'section'>) {
	return (
		<section
			className={cn(
				'min-w-0 overflow-hidden rounded-lg border bg-card',
				className,
			)}
			{...props}
		/>
	)
}

export function Eyebrow({ children }: { children: ReactNode }) {
	return (
		<div className="mt-4 mb-2 font-mono text-[10.5px] leading-none font-medium tracking-[0.09em] text-faint uppercase">
			{children}
		</div>
	)
}

export function Rows({ className, ...props }: ComponentProps<'ul'>) {
	return <ul className={cn('m-0 list-none p-0', className)} {...props} />
}

export function Row({ className, ...props }: ComponentProps<'li'>) {
	return (
		<li
			className={cn(
				'flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 first:border-t-0 hover:bg-secondary',
				className,
			)}
			{...props}
		/>
	)
}

interface TitleProps {
	title: ReactNode
	sub?: ReactNode
	children?: ReactNode
}

export function RowTitle({ title, sub, children }: TitleProps) {
	return (
		<span className="min-w-0 flex-1">
			<b className="block truncate leading-[1.4] font-medium">{title}</b>
			{sub ? (
				<span className="block truncate text-xs leading-[1.4] text-muted-foreground">
					{sub}
				</span>
			) : null}
			{children}
		</span>
	)
}

export function When({ children }: { children: ReactNode }) {
	return (
		<span className="font-mono text-[11.5px] whitespace-nowrap text-faint">
			{children}
		</span>
	)
}

interface EmptyProps {
	icon?: LucideIcon
	title: string
	children?: ReactNode
}

export function PanelEmpty({ icon: Icon, title, children }: EmptyProps) {
	return (
		<div className="px-4 py-[26px] text-center text-muted-foreground">
			{Icon ? (
				<Icon aria-hidden="true" className="mx-auto size-[18px]" />
			) : null}
			<b className="mt-1.5 mb-0.5 block font-semibold text-foreground">
				{title}
			</b>
			{children}
		</div>
	)
}
