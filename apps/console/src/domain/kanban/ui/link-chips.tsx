import { FolderKanban, Users } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'

interface Props {
	client?: Schemas.ClientRefResponse | null
	project?: Schemas.ProjectRefResponse | null
}

export function LinkChips({ client, project }: Props) {
	if (!client && !project) return null
	return (
		<>
			{client ? (
				<Badge variant="outline" className="max-w-full">
					<Users aria-hidden="true" className="size-3 shrink-0" />
					<span className="sr-only">Client : </span>
					<span className="truncate">{client.name}</span>
				</Badge>
			) : null}
			{project ? (
				<Badge variant="outline" className="max-w-full">
					<FolderKanban aria-hidden="true" className="size-3 shrink-0" />
					<span className="sr-only">Projet : </span>
					<span className="truncate">{project.name}</span>
				</Badge>
			) : null}
		</>
	)
}
