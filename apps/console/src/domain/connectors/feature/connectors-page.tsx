import { useState } from 'react'
import { useNotice } from '#/components/layout/notice'
import {
	useAuthorizeMcpConnector,
	useCreateMcpConnector,
	useDisconnectMcpConnector,
	useMcpConnectors,
	useTestMcpConnector,
	useUpdateMcpConnector,
} from '#/domain/connectors/hooks/use-mcp-connectors'
import {
	describeConnectError,
	describeCreateError,
	describeDisconnectError,
	describeTestError,
	describeUpdateError,
} from '#/domain/connectors/model'
import { parseConnectorTab } from '#/domain/connectors/tabs'
import { AddConnectorDialog } from '#/domain/connectors/ui/add-connector-dialog'
import type { ToolsState } from '#/domain/connectors/ui/connector-card'
import { ConnectorsView } from '#/domain/connectors/ui/connectors-view'

interface Props {
	tab: string | undefined
	connectedId?: string | undefined
}

export function ConnectorsPage({ tab, connectedId }: Props) {
	const archived = parseConnectorTab(tab) === 'archives'
	const query = useMcpConnectors(archived)
	const create = useCreateMcpConnector()
	const update = useUpdateMcpConnector()
	const authorize = useAuthorizeMcpConnector()
	const disconnect = useDisconnectMcpConnector()
	const test = useTestMcpConnector()
	const [notice, setNotice] = useNotice()
	const [dialogOpen, setDialogOpen] = useState(false)
	const [createError, setCreateError] = useState<string | null>(null)
	const [tools, setTools] = useState<Record<string, ToolsState>>({})

	const connectors = query.data?.items ?? []
	const busyIds = [
		update.isPending ? update.variables?.path.connector_id : undefined,
		authorize.isPending ? authorize.variables?.path.connector_id : undefined,
		disconnect.isPending ? disconnect.variables?.path.connector_id : undefined,
		test.isPending ? test.variables?.path.connector_id : undefined,
	].filter((id): id is string => id !== undefined)

	const banner = connectedId
		? `Connecteur « ${connectors.find((c) => c.id === connectedId)?.name ?? 'MCP'} » connecté.`
		: null

	const closeDialog = () => {
		setDialogOpen(false)
		setCreateError(null)
	}

	return (
		<>
			<ConnectorsView
				connectors={connectors}
				loading={query.isPending}
				error={query.isError}
				notice={notice}
				banner={banner}
				busyIds={busyIds}
				tools={tools}
				onAdd={() => setDialogOpen(true)}
				onRetry={() => void query.refetch()}
				onConnect={(id) =>
					authorize.mutate(
						{ path: { connector_id: id } },
						{
							onSuccess: (result) =>
								window.location.assign(result.authorization_url),
							onError: (error) => setNotice(describeConnectError(error)),
						},
					)
				}
				onDisconnect={(id) =>
					disconnect.mutate(
						{ path: { connector_id: id } },
						{
							onSuccess: () => {
								setTools(({ [id]: _removed, ...rest }) => rest)
								setNotice('Connecteur déconnecté')
							},
							onError: (error) => setNotice(describeDisconnectError(error)),
						},
					)
				}
				onTest={(id) =>
					test.mutate(
						{ path: { connector_id: id } },
						{
							onSuccess: (result) =>
								setTools((prev) => ({
									...prev,
									[id]: { kind: 'ok', tools: result.tools },
								})),
							onError: (error) =>
								setTools((prev) => ({
									...prev,
									[id]: { kind: 'error', message: describeTestError(error) },
								})),
						},
					)
				}
				onArchive={(id, next) =>
					update.mutate(
						{ path: { connector_id: id }, body: { archived: next } },
						{
							onSuccess: () =>
								setNotice(next ? 'Connecteur archivé' : 'Connecteur restauré'),
							onError: (error) => setNotice(describeUpdateError(error)),
						},
					)
				}
			/>
			<AddConnectorDialog
				open={dialogOpen}
				pending={create.isPending}
				error={createError}
				onClose={closeDialog}
				onSubmit={(draft) => {
					setCreateError(null)
					create.mutate(
						{
							body: {
								url: draft.url,
								...(draft.name ? { name: draft.name } : {}),
							},
						},
						{
							onSuccess: (created) => {
								closeDialog()
								setNotice(`Connecteur « ${created.name} » ajouté`)
							},
							onError: (error) => setCreateError(describeCreateError(error)),
						},
					)
				}}
			/>
		</>
	)
}
