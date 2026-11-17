interface Props {
	name: string
	size?: number
}

const hueOf = (name: string): number =>
	[...name].reduce((acc, char) => (acc * 31 + char.charCodeAt(0)) % 360, 7)

const initialsOf = (name: string): string =>
	name
		.replace(/\s*\(.*\)/, '')
		.split(/\s+/)
		.filter(Boolean)
		.slice(0, 2)
		.map((word) => word[0]?.toUpperCase() ?? '')
		.join('')

export function Avatar({ name, size = 22 }: Props) {
	return (
		<span
			aria-hidden="true"
			className="inline-flex shrink-0 items-center justify-center rounded-full text-[10px] font-semibold text-white"
			style={{
				width: size,
				height: size,
				background: `hsl(${hueOf(name)} 55% 42%)`,
			}}
		>
			{initialsOf(name)}
		</span>
	)
}
