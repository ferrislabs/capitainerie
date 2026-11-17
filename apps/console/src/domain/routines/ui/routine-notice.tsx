interface Props {
	message: string | null
}

export function RoutineNotice({ message }: Props) {
	return (
		<output
			aria-live="polite"
			className={
				message
					? 'rounded-lg border bg-card px-3 py-2 text-[13px] shadow-sm'
					: 'sr-only'
			}
		>
			{message}
		</output>
	)
}
