import { Link } from '@tanstack/react-router'
import { SectionCard } from '#/components/layout/section-card'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { CTL, FieldRow } from '#/domain/agent-team/ui/field-row'
import { HumanAvatar } from '#/domain/agent-team/ui/human-avatar'
import {
	Chip,
	ChipList,
	Empty,
	ROW,
	Rows,
	Ttl,
} from '#/domain/agent-team/ui/rows'
import { Sev, type SevTone } from '#/domain/agent-team/ui/sev'
import { Tbl, Td, Tr } from '#/domain/agent-team/ui/tbl'
import { Toggle } from '#/domain/agent-team/ui/toggle'
import type { DemoConnector, DemoToolRisk } from '#/domain/demo'

const RISK_TONE: Record<DemoToolRisk, SevTone> = {
	lecture: 'ok',
	écriture: 'warn',
	'écriture limitée': 'info',
	'envoi externe': 'crit',
}

interface Props {
	connectors: ReadonlyArray<DemoConnector>
	granted: ReadonlyArray<string>
	form: AgentForm
	onToggle: (connector: string, allowed: boolean) => void
	onChange: (patch: Partial<AgentForm>) => void
}

export function ConnectorsTab({
	connectors,
	granted,
	form,
	onToggle,
	onChange,
}: Props) {
	const allowed = connectors.filter((c) => granted.includes(c.name))
	const blocked = connectors.filter((c) => !granted.includes(c.name))
	const tools = allowed.flatMap((c) =>
		c.tools.map((tool) => ({ tool, connector: c.name })),
	)
	const unavailable = blocked.flatMap((c) =>
		c.tools.map((tool) => ({ tool, connector: c.name })),
	)
	const refused = unavailable.slice(0, 2)
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
				actions={<Link to="/connectors/{-$tab}">Gérer les comptes</Link>}
			>
				<Rows>
					{connectors.map((connector) => {
						const connected = connector.accounts.length > 0
						return (
							<div key={connector.name} className={ROW}>
								<HumanAvatar initials={connector.name[0] ?? ''} size={28} />
								<Ttl
									title={
										<>
											{connector.name}
											<span className="block font-mono text-[12px] font-medium text-muted-foreground">
												{connector.kind}
											</span>
										</>
									}
									sub={connector.description}
								/>
								<Sev tone={connected ? 'ok' : 'info'}>
									{connected ? 'Connecté' : 'Non connecté'}
								</Sev>
								<Toggle
									label={`Autoriser ${connector.name}`}
									checked={granted.includes(connector.name)}
									onChange={(on) => onToggle(connector.name, on)}
								/>
							</div>
						)
					})}
				</Rows>
			</SectionCard>

			<SectionCard
				title="Outils autorisés"
				flush
				actions={
					<span className="text-xs text-muted-foreground">
						{tools.length} {tools.length > 1 ? 'outils' : 'outil'}
					</span>
				}
			>
				{tools.length === 0 ? (
					<Empty title="Aucun outil">Autorise un connecteur ci-dessus.</Empty>
				) : (
					<Tbl head={['Outil', 'Capacité', 'Risque', 'Connecteur']}>
						{tools.map(({ tool, connector }) => (
							<Tr key={`${connector}-${tool.name}`}>
								<Td className="font-mono text-[11.5px]">{tool.name}</Td>
								<Td className="text-muted-foreground">{tool.capability}</Td>
								<Td>
									<Sev tone={RISK_TONE[tool.risk]}>{tool.risk}</Sev>
								</Td>
								<Td className="text-faint">{connector}</Td>
							</Tr>
						))}
					</Tbl>
				)}
			</SectionCard>

			<SectionCard title="Outils indisponibles" flush>
				<ChipList>
					{unavailable.length === 0 ? (
						<Chip>Aucun</Chip>
					) : (
						unavailable.map(({ tool, connector }) => (
							<Chip key={`${connector}-${tool.name}`} title={connector}>
								{tool.name}
							</Chip>
						))
					)}
				</ChipList>
				<p className="m-0 px-3 pb-3 text-muted-foreground">
					Outils que l'agent pourrait nommer mais que Paperclip bloque : leur
					connecteur n'est pas autorisé.
				</p>
			</SectionCard>

			<SectionCard
				title="Accès refusés récemment"
				flush
				actions={
					<span className="text-xs text-muted-foreground">
						7 derniers jours
					</span>
				}
			>
				{refused.length === 0 ? (
					<p className="m-0 p-3 text-muted-foreground">
						Aucun refus : tous les connecteurs sont autorisés.
					</p>
				) : (
					<Rows>
						{refused.map(({ tool, connector }, index) => (
							<div key={`${connector}-${tool.name}`} className={ROW}>
								<Sev tone="crit">Refusé</Sev>
								<Ttl
									title={tool.name}
									titleMono
									sub={`Connecteur « ${connector} » non autorisé pour cet agent`}
								/>
								<span className="text-muted-foreground">
									il y a {index * 3 + 2} h
								</span>
							</div>
						))}
					</Rows>
				)}
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
