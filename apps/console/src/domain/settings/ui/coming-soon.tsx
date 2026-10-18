import type { LucideIcon } from 'lucide-react'
import { Badge } from '#/components/ui/badge'

interface Props {
	icon: LucideIcon
	title: string
	summary: string
	points: ReadonlyArray<string>
}

export function ComingSoon({ icon: Icon, title, summary, points }: Props) {
	return (
		<section
			aria-labelledby="coming-soon-title"
			className="flex flex-col overflow-hidden rounded-lg border bg-card"
		>
			<div className="flex flex-col gap-4 p-6 sm:flex-row sm:items-start sm:gap-5">
				<span className="flex size-11 shrink-0 items-center justify-center rounded-md bg-rail text-rail-foreground-active">
					<Icon aria-hidden="true" className="size-5" />
				</span>
				<div className="flex min-w-0 flex-col gap-2">
					<div className="flex flex-wrap items-center gap-2">
						<h2
							id="coming-soon-title"
							className="text-[15px] leading-tight font-semibold"
						>
							{title}
						</h2>
						<Badge variant="attention">Bientôt disponible</Badge>
					</div>
					<p className="max-w-[65ch] text-[13px] text-muted-foreground">
						{summary}
					</p>
				</div>
			</div>
			<ul className="grid border-t lg:grid-cols-3">
				{points.map((point) => (
					<li
						key={point}
						className="border-b px-6 py-4 text-[13px] last:border-b-0 lg:border-r lg:border-b-0 lg:last:border-r-0"
					>
						{point}
					</li>
				))}
			</ul>
		</section>
	)
}
