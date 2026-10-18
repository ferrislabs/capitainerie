import { Monitor, Moon, Sun } from 'lucide-react'
import type { ThemePreference } from '#/lib/theme'
import { cn } from '#/lib/utils'

const OPTIONS: ReadonlyArray<{
	value: ThemePreference
	label: string
	icon: typeof Sun
}> = [
	{ value: 'light', label: 'Thème clair', icon: Sun },
	{ value: 'dark', label: 'Thème sombre', icon: Moon },
	{ value: 'system', label: 'Thème du système', icon: Monitor },
]

interface Props {
	preference: ThemePreference
	onChange: (preference: ThemePreference) => void
}

export function ThemeToggle({ preference, onChange }: Props) {
	return (
		<fieldset className="flex items-center rounded-md border p-0.5">
			<legend className="sr-only">Thème</legend>
			{OPTIONS.map(({ value, label, icon: Icon }) => {
				const active = value === preference
				return (
					<button
						key={value}
						type="button"
						title={label}
						aria-label={label}
						aria-pressed={active}
						onClick={() => onChange(value)}
						className={cn(
							'flex size-6 items-center justify-center rounded-sm text-muted-foreground transition-colors duration-150 hover:text-foreground',
							'focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
							active && 'bg-secondary text-secondary-foreground',
						)}
					>
						<Icon aria-hidden="true" className="size-3.5" />
					</button>
				)
			})}
		</fieldset>
	)
}
