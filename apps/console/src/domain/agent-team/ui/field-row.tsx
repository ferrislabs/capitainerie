import type { ReactNode } from 'react'

interface Props {
	label: string
	hint?: string
	children: ReactNode
}

export const CTL =
	'w-full min-w-0 rounded-md border bg-background px-[9px] py-[5px] text-foreground focus:border-primary focus:outline-0'

export function FieldRow({ label, hint, children }: Props) {
	return (
		<div className="grid min-w-0 grid-cols-[180px_minmax(0,1fr)] items-center gap-2.5 border-t px-3 py-2 first:border-t-0 [&_select]:max-w-[440px] [&_input:not([type=checkbox])]:max-w-[440px]">
			<span className="text-muted-foreground">
				{label}
				{hint ? (
					<div className="mt-0.5 text-[11.5px] text-faint">{hint}</div>
				) : null}
			</span>
			<div className="min-w-0 [overflow-wrap:anywhere]">{children}</div>
		</div>
	)
}

export function Note({ children }: { children: ReactNode }) {
	return <p className="m-0 px-3 pb-3 text-muted-foreground">{children}</p>
}
