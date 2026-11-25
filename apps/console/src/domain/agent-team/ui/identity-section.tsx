import type { Schemas } from '#/api/api.client'
import { InfoList, InfoRow } from '#/domain/agent-team/ui/field-row'
import { DetailSection } from '#/domain/agent-team/ui/panel'
import { Sev, type SevTone } from '#/domain/agent-team/ui/sev'

const STATUS: Record<
	Schemas.TeamAgentIdentityStatusDto,
	{ label: string; tone: SevTone }
> = {
	pending: { label: 'en attente', tone: 'warn' },
	active: { label: 'active', tone: 'ok' },
	disabled: { label: 'désactivée', tone: 'info' },
}

function explain(identity: Schemas.TeamAgentIdentityResponse): string {
	if (!identity.synced)
		return identity.status === 'disabled'
			? 'FerrisKey désactive le client.'
			: 'FerrisKey réactive le client.'
	if (identity.status === 'pending')
		return 'Le client est en cours de création dans FerrisKey.'
	if (identity.status === 'disabled')
		return "L'agent est archivé : son client ne peut plus obtenir de jeton."
	return "L'agent agit avec son propre client."
}

interface Props {
	identity: Schemas.TeamAgentIdentityResponse | undefined
	loading: boolean
	error: boolean
}

/** Le client FerrisKey de l'agent. Son secret ne quitte jamais Capitainerie. */
export function IdentitySection({ identity, loading, error }: Props) {
	const status = identity ? STATUS[identity.status] : undefined
	return (
		<DetailSection
			title="Identité FerrisKey"
			actions={
				status ? (
					<Sev
						tone={status.tone}
						className="rounded-full px-2.5 py-0.5 text-xs"
					>
						{status.label}
					</Sev>
				) : null
			}
		>
			{error ? (
				<p role="alert" className="m-0 text-signal-danger">
					Impossible de charger l'identité de l'agent.
				</p>
			) : loading || !identity ? (
				<output className="m-0 block text-muted-foreground">
					Chargement de l'identité…
				</output>
			) : (
				<InfoList>
					<InfoRow label="Client" mono>
						{identity.client_id}
					</InfoRow>
					<InfoRow label="État">{explain(identity)}</InfoRow>
					{identity.failure ? (
						<InfoRow label="Dernier échec">{identity.failure}</InfoRow>
					) : null}
				</InfoList>
			)}
		</DetailSection>
	)
}
