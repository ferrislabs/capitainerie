import { Search } from 'lucide-react'
import { Checkbox } from '#/components/ui/checkbox'
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
			<Input
				type="search"
				aria-label={label}
				placeholder={label}
				value={query}
				onChange={(event) => onQueryChange(event.target.value)}
				leading={<Search aria-hidden="true" />}
				clearable
				className="min-w-48 flex-1"
			/>
			<Checkbox
				label="Archivés"
				checked={showArchived}
				onCheckedChange={(checked) => onShowArchivedChange(checked === true)}
				className="text-[13px] text-muted-foreground"
			/>
		</search>
	)
}
