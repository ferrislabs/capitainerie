import type { ComponentProps } from 'react'
import { cn } from '#/lib/utils'

export function DataTable({ className, ...props }: ComponentProps<'table'>) {
	return (
		<div className="min-w-0 overflow-x-auto overflow-y-hidden rounded-lg border bg-card">
			<table
				className={cn('w-full border-collapse text-[13px]', className)}
				{...props}
			/>
		</div>
	)
}

export function Th({ className, ...props }: ComponentProps<'th'>) {
	return (
		<th
			className={cn(
				'border-b px-3 py-[7px] text-left font-mono text-[10.5px] font-medium tracking-[0.06em] whitespace-nowrap text-faint uppercase',
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
				'hover:bg-secondary [&:first-child>td]:border-t-0',
				className,
			)}
			{...props}
		/>
	)
}

export function Td({ className, ...props }: ComponentProps<'td'>) {
	return (
		<td
			className={cn('border-t px-3 py-[7px] align-middle', className)}
			{...props}
		/>
	)
}
