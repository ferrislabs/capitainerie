import { ChevronDown } from 'lucide-react'
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from '#/components/ui/dropdown-menu'
import { toggleValue } from '#/lib/toggle-value'
import { cn } from '#/lib/utils'

interface Props {
	label: string
	options: ReadonlyArray<string>
	selected: ReadonlyArray<string>
	onChange: (values: ReadonlyArray<string>) => void
	labels?: ReadonlyMap<string, string>
}

function summary(
	selected: ReadonlyArray<string>,
	labels: ReadonlyMap<string, string>,
): string | undefined {
	if (selected.length === 0) return undefined
	const [only] = selected
	return selected.length === 1 && only !== undefined
		? (labels.get(only) ?? only)
		: `${selected.length} choix`
}

const NO_LABELS: ReadonlyMap<string, string> = new Map()

export function FilterChip({
	label,
	options,
	selected,
	onChange,
	labels = NO_LABELS,
}: Props) {
	const value = summary(selected, labels)

	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<button
					type="button"
					disabled={options.length === 0}
					className={cn(
						'inline-flex h-8 items-center gap-1.5 rounded-md border px-3 text-[13px] font-medium transition-colors duration-150',
						'hover:bg-muted disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring',
						value === undefined
							? 'border-dashed border-input text-muted-foreground'
							: 'border-input bg-card text-foreground',
					)}
				>
					{label}
					{value !== undefined ? (
						<>
							<span aria-hidden="true" className="text-muted-foreground">
								:
							</span>
							<span className="figures max-w-40 truncate">{value}</span>
						</>
					) : null}
					<ChevronDown aria-hidden="true" className="size-3.5" />
				</button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="start">
				{options.map((option) => (
					<DropdownMenuCheckboxItem
						key={option}
						checked={selected.includes(option)}
						onCheckedChange={() => onChange(toggleValue(selected, option))}
						onSelect={(event) => event.preventDefault()}
					>
						{labels.get(option) ?? option}
					</DropdownMenuCheckboxItem>
				))}
				{selected.length > 0 ? (
					<>
						<DropdownMenuSeparator />
						<DropdownMenuItem onSelect={() => onChange([])}>
							Effacer
						</DropdownMenuItem>
					</>
				) : null}
			</DropdownMenuContent>
		</DropdownMenu>
	)
}
