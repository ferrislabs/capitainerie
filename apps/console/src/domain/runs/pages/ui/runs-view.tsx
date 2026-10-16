import { Play } from 'lucide-react'
import { EmptyState } from '#/components/layout/empty-state'

interface Props {
	heading: string
	emptyTitle: string
	emptyDescription: string
}

export function RunsView({ heading, emptyTitle, emptyDescription }: Props) {
	return (
		<section className="flex flex-col gap-4">
			<h1 className="text-2xl font-semibold">{heading}</h1>
			<EmptyState
				icon={Play}
				title={emptyTitle}
				description={emptyDescription}
			/>
		</section>
	)
}
