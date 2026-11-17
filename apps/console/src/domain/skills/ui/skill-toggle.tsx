import { useState } from 'react'
import { cn } from '#/lib/utils'

interface Props {
	label: string
}

export function SkillToggle({ label }: Props) {
	const [on, setOn] = useState(true)
	return (
		<button
			type="button"
			role="switch"
			aria-checked={on}
			aria-label={label}
			onClick={() => setOn(!on)}
			className={cn(
				'relative h-4 w-7 shrink-0 rounded-lg p-0 transition-colors',
				on ? 'bg-primary' : 'bg-border',
			)}
		>
			<span
				className={cn(
					'absolute top-0.5 left-0.5 size-3 rounded-full transition-transform',
					on ? 'translate-x-3 bg-primary-foreground' : 'bg-foreground',
				)}
			/>
		</button>
	)
}
