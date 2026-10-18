import type * as React from 'react'

import { cn } from '#/lib/utils'

function Table({
	className,
	containerClassName,
	...props
}: React.ComponentProps<'table'> & { containerClassName?: string }) {
	return (
		<div
			data-slot="table-container"
			className={cn('relative w-full overflow-auto', containerClassName)}
		>
			<table
				data-slot="table"
				className={cn(
					'w-full caption-bottom border-separate border-spacing-0 text-[13px]',
					className,
				)}
				{...props}
			/>
		</div>
	)
}

function TableHeader({ className, ...props }: React.ComponentProps<'thead'>) {
	return (
		<thead
			data-slot="table-header"
			className={cn(
				'sticky top-0 z-10 bg-card [&_th]:border-b [&_th]:border-border',
				className,
			)}
			{...props}
		/>
	)
}

function TableBody({ className, ...props }: React.ComponentProps<'tbody'>) {
	return (
		<tbody
			data-slot="table-body"
			className={cn('[&_tr:last-child_td]:border-0', className)}
			{...props}
		/>
	)
}

function TableRow({ className, ...props }: React.ComponentProps<'tr'>) {
	return (
		<tr
			data-slot="table-row"
			className={cn(
				'transition-colors duration-100 [tbody_&]:hover:bg-muted/60',
				className,
			)}
			{...props}
		/>
	)
}

function TableHead({ className, ...props }: React.ComponentProps<'th'>) {
	return (
		<th
			data-slot="table-head"
			className={cn(
				'h-8 bg-card px-3 text-left align-middle text-xs font-medium whitespace-nowrap text-muted-foreground first:pl-4 last:pr-4',
				className,
			)}
			{...props}
		/>
	)
}

function TableCell({ className, ...props }: React.ComponentProps<'td'>) {
	return (
		<td
			data-slot="table-cell"
			className={cn(
				'h-9 border-b border-border/70 px-3 py-1.5 align-middle first:pl-4 last:pr-4',
				className,
			)}
			{...props}
		/>
	)
}

export { Table, TableHeader, TableBody, TableHead, TableRow, TableCell }
