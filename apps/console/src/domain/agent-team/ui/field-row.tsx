import type { ReactNode } from 'react'

interface Props {
	label: string
	hint?: string
	children: ReactNode
}

export function FieldRow({ label, hint, children }: Props) {
	return (
		<div className="grid grid-cols-1 items-center gap-1.5 border-t px-3 py-2.5 first:border-t-0 sm:grid-cols-[14rem_minmax(0,1fr)] sm:gap-3">
			<div className="min-w-0">
				<div className="font-medium">{label}</div>
				{hint ? (
					<div className="text-xs text-muted-foreground">{hint}</div>
				) : null}
			</div>
			<div className="min-w-0">{children}</div>
		</div>
	)
}
