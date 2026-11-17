interface Props {
	message: string | undefined
}

/** Retour discret après une action (remplace les toasts de la maquette). */
export function Notice({ message }: Props) {
	return (
		<output
			aria-live="polite"
			className="block min-h-0 text-[13px] text-signal-success empty:hidden"
		>
			{message}
		</output>
	)
}
