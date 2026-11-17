interface Props {
	message: string | null
}

export function Notice({ message }: Props) {
	return (
		<div
			aria-live="polite"
			className="fixed right-4 bottom-4 z-50 rounded-md border bg-card px-3 py-2 shadow-md data-[empty=true]:hidden"
			data-empty={message === null}
		>
			{message}
		</div>
	)
}
