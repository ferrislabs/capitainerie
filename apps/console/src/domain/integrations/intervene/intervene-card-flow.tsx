import type { Schemas } from '#/api/api.client'
import { useIntegrationInstances } from '#/domain/clients/hooks/use-integration-instances'
import {
	activeInstances,
	type InterveneForm,
	interveneBody,
} from '#/domain/integrations/intervene/grants'
import {
	type InstancesState,
	InterveneCardDialog,
} from '#/domain/integrations/intervene/intervene-card-dialog'
import { useInterveneCard } from '#/domain/integrations/intervene/use-intervene-card'

interface Props {
	card: Schemas.CardResponse | undefined
	onClose: () => void
	onStarted: (runId: string) => void
}

interface ClientProps {
	card: Schemas.CardResponse
	client: Schemas.ClientRefResponse
	onClose: () => void
	onStarted: (runId: string) => void
}

function ClientIntervene({ card, client, onClose, onStarted }: ClientProps) {
	const list = useIntegrationInstances(client.id)
	const intervene = useInterveneCard()
	const instances = activeInstances(list.data?.items ?? [])

	function state(): InstancesState {
		if (list.data) return { kind: 'ready', instances }
		if (list.isError) return { kind: 'error' }
		return { kind: 'loading' }
	}

	function close() {
		intervene.reset()
		onClose()
	}

	function submit(form: InterveneForm) {
		intervene.mutate(
			{
				path: { card_id: card.id },
				body: interveneBody(instances, form, card.version),
			},
			{
				onSuccess: (response) => {
					intervene.reset()
					onStarted(response.run_id)
				},
			},
		)
	}

	return (
		<InterveneCardDialog
			open
			cardTitle={card.title}
			instances={state()}
			errorMessage={intervene.errorMessage}
			isPending={intervene.isPending}
			onRetry={() => void list.refetch()}
			onSubmit={submit}
			onClose={close}
		/>
	)
}

export function InterveneCardFlow({ card, onClose, onStarted }: Props) {
	if (!card?.client) return null

	return (
		<ClientIntervene
			key={card.id}
			card={card}
			client={card.client}
			onClose={onClose}
			onStarted={onStarted}
		/>
	)
}
