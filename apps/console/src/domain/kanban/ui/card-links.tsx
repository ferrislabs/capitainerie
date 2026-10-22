import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Combobox } from '#/components/ui/combobox'
import { Field } from '#/components/ui/field'
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

const LABEL_CLASS = 'text-xs font-normal text-muted-foreground'

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
				<Field label="Client" id="card-client" labelClassName={LABEL_CLASS}>
					<Combobox
						size="sm"
						disabled={isPending || !ready}
						items={clientOptions(card, clients).map((client) => ({
							value: client.id,
							label: client.name,
						}))}
						value={card.client?.id ?? ''}
						noneLabel="Aucun"
						onValueChange={(id) =>
							onLink(linkPatch(card, { field: 'client', id }, projects))
						}
					/>
				</Field>
				<Field label="Projet" id="card-project" labelClassName={LABEL_CLASS}>
					<Combobox
						size="sm"
						disabled={isPending || !ready}
						items={projectOptions(card, projects).map((project) => ({
							value: project.id,
							label: project.name,
						}))}
						value={card.project?.id ?? ''}
						noneLabel="Aucun"
						onValueChange={(id) =>
							onLink(linkPatch(card, { field: 'project', id }, projects))
						}
					/>
				</Field>
			</div>
			{errorMessage ? (
				<InlineAlert tone="danger">{errorMessage}</InlineAlert>
			) : null}
		</section>
	)
}
