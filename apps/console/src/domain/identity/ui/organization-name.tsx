import { Building2 } from 'lucide-react'
import { Skeleton } from '#/components/ui/skeleton'

interface Props {
	name: string | undefined
	isPending: boolean
	isError: boolean
}

export function OrganizationName({ name, isPending, isError }: Props) {
	return (
		<div className="flex min-w-0 items-center gap-2 text-[13px]">
			<Building2
				aria-hidden="true"
				className="size-4 shrink-0 text-muted-foreground"
			/>
			<span className="sr-only">Organisation :</span>
			{isError ? (
				<span role="alert" className="text-signal-danger">
					Organisation indisponible
				</span>
			) : isPending || !name ? (
				<>
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-3.5 w-32" />
				</>
			) : (
				<span className="truncate font-semibold">{name}</span>
			)}
		</div>
	)
}
