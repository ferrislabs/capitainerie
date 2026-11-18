import { Link } from '@tanstack/react-router'
import type { Schemas } from '#/api/api.client'
import { SectionCard } from '#/components/layout/section-card'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { CTL, FieldRow } from '#/domain/agent-team/ui/field-row'
import { HumanAvatar } from '#/domain/agent-team/ui/human-avatar'
import { Empty, ROW, Rows, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
import { Toggle } from '#/domain/agent-team/ui/toggle'
import { hostOf } from '#/domain/connectors/model'

interface Props {
	connectors: ReadonlyArray<Schemas.McpConnectorResponse>
	loading: boolean
	error: boolean
	granted: ReadonlyArray<string>
	form: AgentForm
	onToggle: (slug: string, allowed: boolean) => void
	onChange: (patch: Partial<AgentForm>) => void
}

export function ConnectorsTab({
	connectors,
	loading,
	error,
	granted,
	form,
	onToggle,
	onChange,
}: Props) {
	const orphans = granted.filter(
		(slug) => !connectors.some((c) => c.slug === slug),
	)
	return (
		<>
			<p className="mb-2.5 mt-0 text-muted-foreground">
				Refus par défaut : cet agent n'a accès qu'aux connecteurs que tu
				autorises ci-dessous. Le prompt peut restreindre cette liste, jamais
				l'étendre.
			</p>
			<SectionCard
				title="Connecteurs de l'agent"
				flush
				actions={<Link to="/connectors/{-$tab}">Gérer les connecteurs</Link>}
			>
				{error ? (
					<p role="alert" className="m-0 p-3 text-signal-danger">
						Impossible de charger les connecteurs.
					</p>
				) : loading ? (
					<output className="m-0 block p-3 text-muted-foreground">
						Chargement des connecteurs…
					</output>
				) : connectors.length === 0 && orphans.length === 0 ? (
					<Empty title="Aucun connecteur">
						Ajoute un serveur MCP depuis la page des connecteurs.
					</Empty>
				) : (
					<Rows>
						{connectors.map((connector) => (
							<div key={connector.id} className={ROW}>
								<HumanAvatar initials={connector.name[0] ?? ''} size={28} />
								<Ttl
									title={
										<>
											{connector.name}
											<span className="block font-mono text-[12px] font-medium text-muted-foreground">
												{hostOf(connector.url)}
											</span>
										</>
									}
								/>
								<Sev tone={connector.connected ? 'ok' : 'info'}>
									{connector.connected ? 'Connecté' : 'Non connecté'}
								</Sev>
								<Toggle
									label={`Autoriser ${connector.name}`}
									checked={granted.includes(connector.slug)}
									onChange={(on) => onToggle(connector.slug, on)}
								/>
							</div>
						))}
						{orphans.map((slug) => (
							<div key={slug} className={ROW}>
								<HumanAvatar initials={slug[0] ?? ''} size={28} />
								<Ttl title={slug} sub="Connecteur introuvable ou archivé" />
								<Toggle
									label={`Autoriser ${slug}`}
									checked
									onChange={(on) => onToggle(slug, on)}
								/>
							</div>
						))}
					</Rows>
				)}
				{connectors.some((c) => !c.connected && granted.includes(c.slug)) ? (
					<p className="m-0 px-3 pb-3 text-muted-foreground">
						Tu n'as pas connecté ton compte à certains connecteurs autorisés :
						l'agent ne pourra pas s'en servir en ton nom tant que tu ne t'y es
						pas connecté.
					</p>
				) : null}
			</SectionCard>

			<SectionCard title="Identité GitHub" flush>
				<FieldRow label="Compte utilisé" hint="Au démarrage du run">
					<select
						aria-label="Compte GitHub utilisé"
						className={CTL}
						value={form.gh}
						onChange={(event) => onChange({ gh: event.target.value })}
					>
						<option value="person">
							Compte GitHub de la personne responsable
						</option>
						<option value="own">Compte dédié</option>
					</select>
				</FieldRow>
			</SectionCard>
		</>
	)
}
