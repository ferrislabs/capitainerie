import { Link } from '@tanstack/react-router'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	buildValidationRequest,
	validationTitle,
} from '#/domain/approvals/validation-request'
import {
	addApproval,
	type DemoSource,
	demoStore,
	grantConnector,
	shortName,
	subAgentIds,
	useDemoState,
} from '#/domain/demo'
import {
	crmConnector,
	exportBlocked,
	exportPayload,
	requiredAccess,
} from '#/domain/sources/access'
import { CRITERIA, decide, PRESETS } from '#/domain/sources/criteria'
import { DOSSIER_SEGMENTS, DOSSIERS } from '#/domain/sources/mock/dossiers'
import {
	ACTIONS,
	DEDUPE_KEYS,
	DESTINATIONS,
	pipelineActions,
	unusedCriteria,
	usePipeline,
} from '#/domain/sources/mock/pipeline-store'
import { AccessTable } from '#/domain/sources/ui/access-table'
import { ExportSection } from '#/domain/sources/ui/export-section'
import { FlowStrip } from '#/domain/sources/ui/flow-strip'
import { PipelineCriteria } from '#/domain/sources/ui/pipeline-criteria'
import {
	DossierPanel,
	VerdictPanel,
} from '#/domain/sources/ui/pipeline-dossier'
import { resultLabel } from '#/domain/sources/ui/result-badge'

interface Props {
	source: DemoSource
	notify: (message: string) => void
}

export function PipelineTab({ source, notify }: Props) {
	const config = usePipeline(source.id)
	const agents = useDemoState((s) => s.agents)
	const grants = useDemoState((s) => s.grants)
	const approvals = useDemoState((s) => s.approvals)
	const state = { ...demoStore.state, agents, grants }
	const dossier = DOSSIERS[config.dossier] ?? Object.values(DOSSIERS)[0]
	if (!dossier) return null
	const decision = decide(config.criteria, dossier)
	const crm = crmConnector(config.destination)
	const access = requiredAccess(state, source.agentIds, config.destination)
	const crmRow = access[access.length - 1]
	const crmAgents = crmRow?.installed ? crmRow.agentIds.length : 0
	const blocked = exportBlocked(decision.verdict, crm, crmAgents)
	const payload = blocked
		? null
		: exportPayload(dossier, config.dossier, config.dry, decision.verdict)
	const firstId = source.agentIds[0]
	const first = agents.find((a) => a.id === firstId)
	const subs = firstId
		? subAgentIds(state, firstId)
				.map((id) => agents.find((a) => a.id === id))
				.filter((a) => a !== undefined)
		: []
	const message = blocked
		? `Export bloqué : aucun agent n'a accès au connecteur ${crm ?? config.destination}`
		: payload
			? decision.verdict === 'qualified'
				? 'Charge envoyée au CRM'
				: 'Charge envoyée après validation'
			: 'Aucun export pour ce dossier'
	const id = source.id
	const requested = approvals.some(
		(a) => a.title === validationTitle(dossier.raison, source.name),
	)
	const requestValidation = () => {
		const reasons = decision.rows
			.filter((r) => r.result !== 'ok' && r.criterion.onFail !== 'ignore')
			.map(
				(r) =>
					`${CRITERIA[r.criterion.id].label} : ${resultLabel(r.result).toLowerCase()}`,
			)
		const request = buildValidationRequest({
			approvals: demoStore.state.approvals,
			raison: dossier.raison,
			sourceName: source.name,
			reasons,
			requestedBy: firstId ?? null,
		})
		if (!request) {
			notify('Demande déjà envoyée')
			return
		}
		addApproval(request)
		notify('Demande de validation envoyée')
	}
	return (
		<div className="flex flex-col gap-4">
			<SectionCard title="Chemin complet">
				<FlowStrip
					steps={[
						{ title: '1. Source', body: source.patterns.join('  ') || '—' },
						{
							title: '2. Recherche (IA)',
							body: `${first ? shortName(first.name) : 'Aucun agent'} · dossier structuré${
								subs.length > 0
									? ` · délègue à ${subs.map((a) => shortName(a.name)).join(', ')}`
									: ''
							}`,
						},
						{
							title: '3. Décision (règles)',
							body: `${config.criteria.filter((c) => c.enabled).length} critères actifs`,
						},
						{
							title: '4. Export (MCP)',
							body: `${config.destination} · ${config.dry ? 'essai à blanc' : 'écriture réelle'}`,
						},
					]}
				/>
			</SectionCard>
			<div className="grid items-start gap-4 lg:grid-cols-2">
				<SectionCard title="Dossier produit par l'agent" flush>
					<DossierPanel
						dossier={dossier}
						segments={DOSSIER_SEGMENTS}
						selected={config.dossier}
						onSelect={(value) => pipelineActions.set(id, { dossier: value })}
					/>
				</SectionCard>
				<SectionCard title="Résultat de la décision" flush>
					<VerdictPanel decision={decision} />
					{decision.verdict === 'review' ? (
						<div className="flex flex-wrap items-center gap-3 px-3 pb-3">
							<Button type="button" size="sm" onClick={requestValidation}>
								Demander une validation
							</Button>
							{requested ? (
								<Link to="/approvals" className="text-sm hover:underline">
									Voir les approbations
								</Link>
							) : null}
						</div>
					) : null}
				</SectionCard>
			</div>
			<SectionCard
				title="3. Critères de décision"
				description="Qualifié si tous les critères actifs sont remplis"
				flush
			>
				<PipelineCriteria
					criteria={config.criteria}
					dossier={dossier}
					available={unusedCriteria(config.criteria)}
					presets={Object.keys(PRESETS)}
					onToggle={(i) => pipelineActions.toggle(id, i)}
					onParam={(i, p) => pipelineActions.setParam(id, i, p)}
					onOnFail={(i, f) => pipelineActions.setOnFail(id, i, f)}
					onRemove={(i) => pipelineActions.remove(id, i)}
					onAdd={(c) => pipelineActions.add(id, c)}
					onPreset={(name) => {
						pipelineActions.applyPreset(id, name)
						notify(`Préréglage appliqué : ${name}`)
					}}
				/>
			</SectionCard>
			<SectionCard title="Accès requis (connecteurs)" flush>
				<AccessTable
					rows={access}
					agents={agents}
					grantee={
						first ? { id: first.id, label: shortName(first.name) } : null
					}
					onGrant={(agentId, connector) => {
						grantConnector(agentId, connector)
					}}
				/>
			</SectionCard>
			<SectionCard title="4. Export vers le CRM" flush>
				<ExportSection
					destination={config.destination}
					action={config.action}
					dedupeKey={config.key}
					dry={config.dry}
					destinations={DESTINATIONS}
					actions={ACTIONS}
					keys={DEDUPE_KEYS}
					message={message}
					payload={payload}
					onDestination={(destination) =>
						pipelineActions.set(id, { destination })
					}
					onAction={(action) => pipelineActions.set(id, { action })}
					onKey={(key) => pipelineActions.set(id, { key })}
					onDry={(dry) => {
						pipelineActions.set(id, { dry })
						notify(
							dry
								? "Essai à blanc : rien n'est écrit"
								: 'Écriture réelle dans le CRM',
						)
					}}
				/>
			</SectionCard>
		</div>
	)
}
