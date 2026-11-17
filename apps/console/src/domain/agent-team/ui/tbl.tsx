import type { ReactNode } from 'react'
import { cn } from '#/lib/utils'

export function Tbl({
	head,
	children,
}: {
	head: ReadonlyArray<string>
	children: ReactNode
}) {
	return (
		<div className="overflow-x-auto">
			<table className="w-full border-collapse text-[16px] leading-[normal]">
				<thead>
					<tr>
						{head.map((label) => (
							<th
								key={label || 'actions'}
								className="whitespace-nowrap border-b px-3 py-[7px] text-left font-mono text-[10.5px] font-medium uppercase tracking-[0.06em] text-faint"
							>
								{label}
							</th>
						))}
					</tr>
				</thead>
				<tbody>{children}</tbody>
			</table>
		</div>
	)
}

export function Tr({ children }: { children: ReactNode }) {
	return (
		<tr className="hover:bg-secondary [&:first-child>td]:border-t-0">
			{children}
		</tr>
	)
}

export function Td({
	children,
	className,
	colSpan,
}: {
	children?: ReactNode
	className?: string
	colSpan?: number
}) {
	return (
		<td
			colSpan={colSpan}
			className={cn('border-t px-3 py-[7px] align-middle', className)}
		>
			{children}
		</td>
	)
}
