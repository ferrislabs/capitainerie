import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import {
	clientOptions,
	type LinkPatch,
	linkPatch,
	projectOptions,
} from '#/domain/kanban/link-patch'

interface Props {
	card: Schemas.CardResponse
	clients: ReadonlyArray<Schemas.ClientResponse>
	projects: ReadonlyArray<Schemas.ProjectResponse>
	isPending: boolean
	ready?: boolean
	errorMessage: string | undefined
	onLink: (patch: LinkPatch) => void
}

const SELECT_CLASS =
	'h-8 w-full rounded-md border border-input bg-card px-2 text-[13px] text-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 focus-visible:outline-none disabled:opacity-60'

export function CardLinks({
	card,
	clients,
	projects,
	isPending,
	ready = true,
	errorMessage,
	onLink,
}: Props) {
	return (
		<section className="flex flex-col gap-2">
			<h3 className="text-[15px] font-semibold">Rattachement</h3>
			<div className="grid gap-3 sm:grid-cols-2">
				<label className="flex flex-col gap-1.5 text-xs text-muted-foreground">
					Client
					<select
						aria-label="Client"
						className={SELECT_CLASS}
						disabled={isPending || !ready}
						value={card.client?.id ?? ''}
						onChange={(event) =>
							onLink(
								linkPatch(
									card,
									{ field: 'client', id: event.target.value },
									projects,
								),
							)
						}
					>
						<option value="">Aucun</option>
						{clientOptions(card, clients).map((client) => (
							<option key={client.id} value={client.id}>
								{client.name}
							</option>
						))}
					</select>
				</label>
				<label className="flex flex-col gap-1.5 text-xs text-muted-foreground">
					Projet
					<select
						aria-label="Projet"
						className={SELECT_CLASS}
						disabled={isPending || !ready}
						value={card.project?.id ?? ''}
						onChange={(event) =>
							onLink(
								linkPatch(
									card,
									{ field: 'project', id: event.target.value },
									projects,
								),
							)
						}
					>
						<option value="">Aucun</option>
						{projectOptions(card, projects).map((project) => (
							<option key={project.id} value={project.id}>
								{project.name}
							</option>
						))}
					</select>
				</label>
			</div>
			{errorMessage ? (
				<InlineAlert tone="danger">{errorMessage}</InlineAlert>
			) : null}
		</section>
	)
}
