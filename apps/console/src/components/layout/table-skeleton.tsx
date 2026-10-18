import { Skeleton } from '#/components/ui/skeleton'

interface Props {
	rows?: number
	columns?: number
	label?: string
}

export function TableSkeleton({
	rows = 5,
	columns = 4,
	label = 'Chargement…',
}: Props) {
	return (
		<output className="flex flex-col">
			<span className="sr-only">{label}</span>
			<div className="flex h-8 items-center gap-6 border-b px-4">
				{Array.from({ length: columns }, (_, column) => (
					<Skeleton
						// biome-ignore lint/suspicious/noArrayIndexKey: static placeholder
						key={column}
						className="h-2.5 flex-1 opacity-70"
					/>
				))}
			</div>
			{Array.from({ length: rows }, (_, row) => (
				<div
					// biome-ignore lint/suspicious/noArrayIndexKey: static placeholder
					key={row}
					className="flex h-9 items-center gap-6 border-b border-border/70 px-4 last:border-0"
				>
					{Array.from({ length: columns }, (_, column) => (
						<Skeleton
							// biome-ignore lint/suspicious/noArrayIndexKey: static placeholder
							key={column}
							className="h-3 flex-1"
							style={{ maxWidth: column === 0 ? '16rem' : '7rem' }}
						/>
					))}
				</div>
			))}
		</output>
	)
}
