import type { LucideIcon } from 'lucide-react'
import {
	Card,
	CardContent,
	CardDescription,
	CardTitle,
} from '#/components/ui/card'

interface Props {
	icon: LucideIcon
	title: string
	description: string
}

export function EmptyState({ icon: Icon, title, description }: Props) {
	return (
		<Card>
			<CardContent className="flex flex-col items-center gap-3 py-10 text-center">
				<Icon className="size-8 text-muted-foreground" aria-hidden="true" />
				<CardTitle>{title}</CardTitle>
				<CardDescription>{description}</CardDescription>
			</CardContent>
		</Card>
	)
}
