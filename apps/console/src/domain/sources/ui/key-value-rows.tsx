import type { ReactNode } from 'react'

interface Props {
	rows: ReadonlyArray<readonly [string, ReactNode]>
}

export function KeyValueRows({ rows }: Props) {
	return (
		<dl className="divide-y">
			{rows.map(([label, value]) => (
				<div key={label} className="flex gap-4 px-3 py-2">
					<dt className="w-44 shrink-0 text-muted-foreground">{label}</dt>
					<dd className="min-w-0 break-words">{value}</dd>
				</div>
			))}
		</dl>
	)
}
