import { cn } from '#/lib/utils'

interface Props {
	checked: boolean
	label: string
	onChange: (checked: boolean) => void
}

export function Toggle({ checked, label, onChange }: Props) {
	return (
		<button
			type="button"
			role="switch"
			aria-checked={checked}
			aria-label={label}
			onClick={() => onChange(!checked)}
			className={cn(
				"relative h-4 w-7 shrink-0 rounded-lg border-0 p-0 after:absolute after:top-0.5 after:left-0.5 after:size-3 after:rounded-full after:bg-foreground after:transition-transform after:duration-150 after:content-['']",
				checked ? 'bg-primary after:translate-x-3 after:bg-white' : 'bg-border',
			)}
		/>
	)
}
