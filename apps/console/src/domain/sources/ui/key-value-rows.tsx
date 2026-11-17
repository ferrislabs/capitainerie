import type { ReactNode } from 'react'
import { Kv } from '#/domain/sources/ui/mockup'

interface Props {
	rows: ReadonlyArray<readonly [string, ReactNode]>
}

export function KeyValueRows({ rows }: Props) {
	return (
		<div>
			{rows.map(([label, value]) => (
				<Kv key={label} label={label}>
					{value}
				</Kv>
			))}
		</div>
	)
}
