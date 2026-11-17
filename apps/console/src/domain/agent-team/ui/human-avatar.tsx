interface Props {
	initials: string
	size?: number
}

export function HumanAvatar({ initials, size = 20 }: Props) {
	return (
		<span
			aria-hidden="true"
			className="inline-grid shrink-0 place-items-center rounded-full bg-secondary text-[9px] font-semibold text-muted-foreground"
			style={{ width: size, height: size }}
		>
			{initials}
		</span>
	)
}
