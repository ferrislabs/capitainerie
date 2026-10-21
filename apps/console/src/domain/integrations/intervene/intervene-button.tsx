import { useId } from 'react'
import type { Schemas } from '#/api/api.client'
import { Button } from '#/components/ui/button'
import { useIntegrationInstances } from '#/domain/clients/hooks/use-integration-instances'
import {
	type InterveneAvailability,
	interveneAvailability,
} from '#/domain/integrations/intervene/availability'
import { cn } from '#/lib/utils'

interface Props {
	card: Schemas.CardResponse
	disabled?: boolean
	className?: string
	buttonClassName?: string
	onIntervene: (card: Schemas.CardResponse) => void
}

interface TriggerProps {
	availability: InterveneAvailability
	disabled: boolean
	className: string | undefined
	buttonClassName: string | undefined
	onClick: () => void
}

function Trigger({
	availability,
	disabled,
	className,
	buttonClassName,
	onClick,
}: TriggerProps) {
	const reasonId = useId()
	const reason =
		availability.kind === 'blocked' ? availability.reason : undefined

	return (
		<div className={cn('flex flex-col gap-1', className)}>
			<Button
				type="button"
				size="sm"
				variant="outline"
				className={buttonClassName}
				disabled={disabled || availability.kind !== 'ready'}
				aria-describedby={reason ? reasonId : undefined}
				onClick={onClick}
			>
				Confier à l’Intervenant
			</Button>
			{reason ? (
				<p id={reasonId} className="text-[11px] text-muted-foreground">
					{reason}
				</p>
			) : null}
		</div>
	)
}

function ClientInterveneButton({
	card,
	client,
	disabled = false,
	className,
	buttonClassName,
	onIntervene,
}: Props & { client: Schemas.ClientRefResponse }) {
	const list = useIntegrationInstances(client.id)
	const availability: InterveneAvailability = list.isError
		? { kind: 'ready' }
		: interveneAvailability(client, list.data?.items)

	return (
		<Trigger
			availability={availability}
			disabled={disabled}
			className={className}
			buttonClassName={buttonClassName}
			onClick={() => onIntervene(card)}
		/>
	)
}

export function InterveneButton(props: Props) {
	const { card, disabled = false, className, buttonClassName } = props
	if (!card.client) {
		return (
			<Trigger
				availability={interveneAvailability(null, undefined)}
				disabled={disabled}
				className={className}
				buttonClassName={buttonClassName}
				onClick={() => props.onIntervene(card)}
			/>
		)
	}
	return <ClientInterveneButton {...props} client={card.client} />
}
