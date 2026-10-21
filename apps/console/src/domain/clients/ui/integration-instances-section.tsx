import { Plug, Plus } from 'lucide-react'
import { Fragment, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { useIntegrationInstances } from '#/domain/clients/hooks/use-integration-instances'
import {
	useArchiveIntegrationInstance,
	useCheckIntegrationInstance,
	useCreateIntegrationInstance,
	useUpdateIntegrationInstance,
} from '#/domain/clients/hooks/use-integration-mutations'
import { integrationFailureMessage } from '#/domain/clients/integration-errors'
import {
	type IntegrationValues,
	integrationBody,
	integrationPatch,
} from '#/domain/clients/integration-form'
import {
	KIND_LABELS,
	serverVersionLabel,
} from '#/domain/clients/integration-labels'
import { ArchiveIntegrationDialog } from '#/domain/clients/ui/archive-integration-dialog'
import { IntegrationInstanceDialog } from '#/domain/clients/ui/integration-instance-dialog'
import { useIsAdmin } from '#/domain/identity/hooks/use-is-admin'
import { EnvironmentBadge } from '#/domain/integrations/environment-badge'

type Instance = Schemas.IntegrationInstanceResponse
type CheckResult = Schemas.CheckIntegrationInstanceResponse
type Panel = 'add' | 'edit' | 'archive'

const dateFormat = new Intl.DateTimeFormat('fr-FR', {
	dateStyle: 'short',
	timeStyle: 'short',
})

function checkedAt(iso: string | null | undefined): string {
	if (!iso) return 'Jamais testée'
	const date = new Date(iso)
	return Number.isNaN(date.getTime()) ? '—' : dateFormat.format(date)
}

function CheckOutcome({
	instance,
	result,
}: {
	instance: Instance
	result: CheckResult
}) {
	const version = serverVersionLabel(instance.kind, result.server_version)
	return (
		<div className="flex flex-col gap-2">
			{result.ok ? (
				<InlineAlert tone="success">
					Connexion réussie{version ? `. Serveur : ${version}` : ''}.
				</InlineAlert>
			) : (
				<InlineAlert tone="danger">
					{result.error
						? integrationFailureMessage(result.error.code, result.error.message)
						: 'Connexion impossible.'}
				</InlineAlert>
			)}
			{result.warning ? (
				<InlineAlert tone="info">{result.warning}</InlineAlert>
			) : null}
		</div>
	)
}

interface Props {
	clientId: string
	archived: boolean
}

export function IntegrationInstancesSection({ clientId, archived }: Props) {
	const isAdmin = useIsAdmin()
	const list = useIntegrationInstances(clientId)
	const create = useCreateIntegrationInstance()
	const update = useUpdateIntegrationInstance()
	const check = useCheckIntegrationInstance()
	const archive = useArchiveIntegrationInstance()
	const [panel, setPanel] = useState<Panel>()
	const [editing, setEditing] = useState<Instance>()
	const [archiving, setArchiving] = useState<Instance>()
	const [results, setResults] = useState<Record<string, CheckResult>>({})
	const checkingId = check.isPending ? check.variables.path.id : undefined
	const failedId = check.isError ? check.variables.path.id : undefined

	function closePanel() {
		create.reset()
		update.reset()
		archive.reset()
		setPanel(undefined)
	}

	function submitCreate(values: IntegrationValues) {
		create.mutate(
			{ path: { client_id: clientId }, body: integrationBody(values) },
			{ onSuccess: closePanel },
		)
	}

	function submitUpdate(values: IntegrationValues) {
		if (!editing) return
		const body = integrationPatch(values, editing)
		if (Object.keys(body).length === 0) {
			closePanel()
			return
		}
		update.mutate({ path: { id: editing.id }, body }, { onSuccess: closePanel })
	}

	function runCheck(instance: Instance) {
		check.mutate(
			{ path: { id: instance.id } },
			{
				onSuccess: (result) =>
					setResults((current) => ({ ...current, [instance.id]: result })),
			},
		)
	}

	function confirmArchive() {
		if (!archiving) return
		archive.mutate({ path: { id: archiving.id } }, { onSuccess: closePanel })
	}

	return (
		<SectionCard
			title="Intégrations"
			bare
			actions={
				isAdmin ? (
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={archived}
						onClick={() => {
							create.reset()
							setPanel('add')
						}}
					>
						<Plus aria-hidden="true" />
						Ajouter une intégration
					</Button>
				) : null
			}
		>
			{list.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les intégrations.
				</InlineAlert>
			) : list.isPending ? (
				<output className="flex flex-col gap-2">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-10 w-full rounded-lg" />
				</output>
			) : list.data.items.length === 0 ? (
				<EmptyState
					icon={Plug}
					title="Aucune intégration"
					description="Ce client n’a pas encore d’instance Odoo ou n8n."
				/>
			) : (
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>Nom</TableHead>
							<TableHead>Type</TableHead>
							<TableHead>Environnement</TableHead>
							<TableHead>URL</TableHead>
							<TableHead>Version</TableHead>
							<TableHead>Dernier test</TableHead>
							<TableHead>Clé</TableHead>
							{isAdmin ? (
								<TableHead>
									<span className="sr-only">Actions</span>
								</TableHead>
							) : null}
						</TableRow>
					</TableHeader>
					<TableBody>
						{list.data.items.map((instance) => {
							const result = results[instance.id]
							const outcome = checkingId === instance.id ? undefined : result
							const failure =
								failedId === instance.id ? check.errorMessage : undefined
							return (
								<Fragment key={instance.id}>
									<TableRow>
										<TableCell className="font-medium">
											{instance.name}
										</TableCell>
										<TableCell className="whitespace-nowrap">
											{KIND_LABELS[instance.kind]}
										</TableCell>
										<TableCell>
											<EnvironmentBadge environment={instance.environment} />
										</TableCell>
										<TableCell
											className="max-w-[18rem] truncate font-mono text-xs"
											title={instance.base_url}
										>
											{instance.base_url}
										</TableCell>
										<TableCell className="whitespace-nowrap">
											{serverVersionLabel(
												instance.kind,
												instance.server_version,
											) ?? '—'}
										</TableCell>
										<TableCell className="whitespace-nowrap text-muted-foreground figures">
											{checkedAt(instance.checked_at)}
										</TableCell>
										<TableCell className="font-mono text-xs text-muted-foreground">
											{instance.secret_preview}
										</TableCell>
										{isAdmin ? (
											<TableCell>
												<div className="flex justify-end gap-2">
													<Button
														type="button"
														size="sm"
														variant="outline"
														disabled={check.isPending}
														onClick={() => runCheck(instance)}
													>
														{checkingId === instance.id
															? 'Test de la connexion…'
															: 'Tester la connexion'}
													</Button>
													<Button
														type="button"
														size="sm"
														variant="outline"
														onClick={() => {
															update.reset()
															setEditing(instance)
															setPanel('edit')
														}}
													>
														Modifier
													</Button>
													<Button
														type="button"
														size="sm"
														variant="outline"
														onClick={() => {
															archive.reset()
															setArchiving(instance)
															setPanel('archive')
														}}
													>
														Archiver
													</Button>
												</div>
											</TableCell>
										) : null}
									</TableRow>
									{outcome || failure ? (
										<TableRow>
											<TableCell colSpan={isAdmin ? 8 : 7}>
												{outcome ? (
													<CheckOutcome instance={instance} result={outcome} />
												) : (
													<InlineAlert tone="danger">{failure}</InlineAlert>
												)}
											</TableCell>
										</TableRow>
									) : null}
								</Fragment>
							)
						})}
					</TableBody>
				</Table>
			)}
			{isAdmin ? (
				<>
					<IntegrationInstanceDialog
						open={panel === 'add'}
						isPending={create.isPending}
						errorMessage={create.errorMessage}
						onClose={closePanel}
						onSubmit={submitCreate}
					/>
					<IntegrationInstanceDialog
						open={panel === 'edit'}
						instance={editing}
						isPending={update.isPending}
						errorMessage={update.errorMessage}
						onClose={closePanel}
						onSubmit={submitUpdate}
					/>
					<ArchiveIntegrationDialog
						open={panel === 'archive'}
						instance={archiving}
						isPending={archive.isPending}
						errorMessage={archive.errorMessage}
						onClose={closePanel}
						onConfirm={confirmArchive}
					/>
				</>
			) : null}
		</SectionCard>
	)
}
