import { Search } from 'lucide-react'
import { Input } from '#/components/ui/input'

interface Props {
	label: string
	query: string
	showArchived: boolean
	onQueryChange: (value: string) => void
	onShowArchivedChange: (value: boolean) => void
}

export function DirectoryToolbar({
	label,
	query,
	showArchived,
	onQueryChange,
	onShowArchivedChange,
}: Props) {
	return (
		<search aria-label={label} className="flex flex-wrap items-center gap-3">
			<div className="relative min-w-48 flex-1">
				<Search
					aria-hidden="true"
					className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
				/>
				<Input
					type="search"
					aria-label={label}
					placeholder={label}
					value={query}
					onChange={(event) => onQueryChange(event.target.value)}
					className="pl-8"
				/>
			</div>
			<label className="flex items-center gap-2 text-[13px] text-muted-foreground">
				<input
					type="checkbox"
					checked={showArchived}
					onChange={(event) => onShowArchivedChange(event.target.checked)}
				/>
				Archivés
			</label>
		</search>
	)
}
