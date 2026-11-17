import type { ReactNode } from 'react'

export interface FlowStep {
	title: string
	body: ReactNode
}

interface Props {
	steps: ReadonlyArray<FlowStep>
}

export function FlowStrip({ steps }: Props) {
	return (
		<ol className="flex flex-wrap items-stretch gap-2">
			{steps.map((step, i) => (
				<li
					key={step.title}
					className="flex min-w-40 flex-1 items-center gap-2"
				>
					<div className="flex min-w-0 flex-1 flex-col gap-0.5 rounded-md border bg-background p-2">
						<b className="font-medium">{step.title}</b>
						<span className="break-words text-muted-foreground">
							{step.body}
						</span>
					</div>
					{i < steps.length - 1 ? (
						<span aria-hidden="true" className="text-muted-foreground">
							→
						</span>
					) : null}
				</li>
			))}
		</ol>
	)
}
