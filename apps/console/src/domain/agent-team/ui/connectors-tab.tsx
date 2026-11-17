import { Link } from '@tanstack/react-router'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { NativeSelect } from '#/components/ui/native-select'
import { Switch } from '#/components/ui/switch'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { AgentForm } from '#/domain/agent-team/agent-config'
import { FieldRow } from '#/domain/agent-team/ui/field-row'
import type { DemoConnector, DemoToolRisk } from '#/domain/demo'

const RISK_VARIANT: Record<
	DemoToolRisk,
	'success' | 'attention' | 'neutral' | 'danger'
> = {
	lecture: 'success',
	écriture: 'attention',
	'écriture limitée': 'neutral',
	'envoi externe': 'danger',
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
		<div className="flex flex-col gap-4">
			<p className="text-muted-foreground">
				Refus par défaut : cet agent n'a accès qu'aux connecteurs que tu
				autorises ci-dessous. Le prompt peut restreindre cette liste, jamais
				l'étendre.
			</p>
			<SectionCard
				title="Connecteurs de l'agent"
				flush
				actions={
					<Link to="/connectors/{-$tab}" className="text-link hover:underline">
						Gérer les comptes
					</Link>
				}
			>
				<ul className="divide-y">
					{connectors.map((connector) => {
						const connected = connector.accounts.length > 0
						return (
							<li
								key={connector.name}
								className="flex items-center gap-3 px-3 py-2.5"
							>
								<span className="min-w-0 flex-1">
									<b className="flex items-center gap-1.5 font-medium">
										{connector.name}
										<Badge variant="outline">{connector.kind}</Badge>
									</b>
									<span className="block truncate text-xs text-muted-foreground">
										{connector.description}
									</span>
								</span>
								<Badge variant={connected ? 'success' : 'neutral'}>
									{connected ? 'Connecté' : 'Non connecté'}
								</Badge>
								<Switch
									aria-label={`Autoriser ${connector.name}`}
									checked={granted.includes(connector.name)}
									onCheckedChange={(on) => onToggle(connector.name, on)}
								/>
							</li>
						)
					})}
				</ul>
			</SectionCard>

			<SectionCard
				title="Outils autorisés"
				bare
				actions={
					<span className="text-xs text-muted-foreground">
						{tools.length} {tools.length > 1 ? 'outils' : 'outil'}
					</span>
				}
			>
				{tools.length === 0 ? (
					<p className="rounded-lg border bg-card px-3 py-4 text-muted-foreground">
						Aucun outil. Autorise un connecteur ci-dessus.
					</p>
				) : (
					<div className="overflow-hidden rounded-lg border bg-card">
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>Outil</TableHead>
									<TableHead>Capacité</TableHead>
									<TableHead>Risque</TableHead>
									<TableHead>Connecteur</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{tools.map(({ tool, connector }) => (
									<TableRow key={`${connector}-${tool.name}`}>
										<TableCell className="font-mono">{tool.name}</TableCell>
										<TableCell className="text-muted-foreground">
											{tool.capability}
										</TableCell>
										<TableCell>
											<Badge variant={RISK_VARIANT[tool.risk]}>
												{tool.risk}
											</Badge>
										</TableCell>
										<TableCell className="text-muted-foreground">
											{connector}
										</TableCell>
									</TableRow>
								))}
							</TableBody>
						</Table>
					</div>
				)}
			</SectionCard>

			<SectionCard
				title="Outils indisponibles"
				description="Outils que l'agent pourrait nommer mais que Paperclip bloque : leur connecteur n'est pas autorisé."
			>
				{unavailable.length === 0 ? (
					<span className="text-muted-foreground">Aucun</span>
				) : (
					<ul className="flex flex-wrap gap-1.5">
						{unavailable.map(({ tool, connector }) => (
							<li key={`${connector}-${tool.name}`} title={connector}>
								<Badge variant="muted" className="font-mono">
									{tool.name}
								</Badge>
							</li>
						))}
					</ul>
				)}
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
					<p className="px-3 py-4 text-muted-foreground">
						Aucun refus : tous les connecteurs sont autorisés.
					</p>
				) : (
					<ul className="divide-y">
						{refused.map(({ tool, connector }, index) => (
							<li
								key={`${connector}-${tool.name}`}
								className="flex items-center gap-3 px-3 py-2.5"
							>
								<Badge variant="danger">Refusé</Badge>
								<span className="min-w-0 flex-1">
									<b className="block font-mono font-medium">{tool.name}</b>
									<span className="text-xs text-muted-foreground">
										Connecteur « {connector} » non autorisé pour cet agent
									</span>
								</span>
								<span className="text-muted-foreground">
									il y a {index * 3 + 2} h
								</span>
							</li>
						))}
					</ul>
				)}
			</SectionCard>

			<SectionCard title="Identité GitHub" flush>
				<FieldRow label="Compte utilisé" hint="Au démarrage du run">
					<NativeSelect
						aria-label="Compte GitHub utilisé"
						value={form.gh}
						onChange={(event) => onChange({ gh: event.target.value })}
					>
						<option value="person">
							Compte GitHub de la personne responsable
						</option>
						<option value="own">Compte dédié</option>
					</NativeSelect>
				</FieldRow>
			</SectionCard>
		</div>
	)
}
