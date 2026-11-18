import { Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import {
	type BudgetDraft,
	draftsOf,
	formatCents,
	microsToCents,
	saveRequestOf,
} from '#/domain/audit/model'
import { Panel, PanelEmpty, Row, Rows, RowTitle } from '#/domain/audit/ui/rows'
import { Sev } from '#/domain/audit/ui/sev'
import { CTL } from '#/domain/sources/ui/kit'

interface AgentOption {
	id: string
	name: string
}

interface Props {
	budgets: Schemas.BudgetsResponse
	agents: ReadonlyArray<AgentOption>
	saving: boolean
	error: string | null
	onSave: (request: Schemas.SaveBudgetsRequest, done: () => void) => void
}

const scopeLabel = (budget: Schemas.BudgetResponse): string =>
	budget.scope === 'organization'
		? 'Organisation'
		: (budget.agent_name ?? 'Agent')

function BudgetRows({ budgets }: { budgets: Schemas.BudgetsResponse }) {
	if (budgets.items.length === 0) {
		return (
			<PanelEmpty title="Aucun budget">
				Aucune limite mensuelle n'est définie.
			</PanelEmpty>
		)
	}
	return (
		<Rows>
			{budgets.items.map((budget) => {
				const label = scopeLabel(budget)
				const width = Math.min(100, Math.max(budget.percent, 1))
				return (
					<Row key={budget.agent_id ?? budget.scope}>
						<RowTitle
							title={label}
							sub={`${formatCents(microsToCents(budget.spent_micros))} sur ${formatCents(budget.monthly_limit_cents)} ce mois · alerte à ${budget.alert_threshold_percent} %`}
						/>
						{budget.exceeded ? (
							<Sev tone="crit">Dépassé</Sev>
						) : budget.alert ? (
							<Sev tone="warn">Seuil atteint</Sev>
						) : null}
						<div
							role="progressbar"
							aria-label={`Budget consommé : ${label}`}
							aria-valuemin={0}
							aria-valuemax={100}
							aria-valuenow={Math.min(100, budget.percent)}
							className="h-1.5 w-[180px] min-w-[60px] overflow-hidden rounded-[3px] bg-secondary"
						>
							<i
								className={
									budget.exceeded
										? 'block h-full rounded-[3px] bg-signal-danger'
										: 'block h-full rounded-[3px] bg-primary'
								}
								style={{ width: `${width}%` }}
							/>
						</div>
						<span className="figures w-10 text-right font-mono text-[11.5px] text-muted-foreground">
							{budget.percent} %
						</span>
					</Row>
				)
			})}
		</Rows>
	)
}

interface EditorProps {
	initial: ReadonlyArray<BudgetDraft>
	agents: ReadonlyArray<AgentOption>
	saving: boolean
	onCancel: () => void
	onSubmit: (drafts: ReadonlyArray<BudgetDraft>) => string | null
}

function BudgetEditor({
	initial,
	agents,
	saving,
	onCancel,
	onSubmit,
}: EditorProps) {
	const [drafts, setDrafts] = useState<BudgetDraft[]>([...initial])
	const [invalid, setInvalid] = useState<string | null>(null)
	const hasOrganization = drafts.some((draft) => draft.agentId === null)
	const available = agents.filter(
		(agent) => !drafts.some((draft) => draft.agentId === agent.id),
	)
	const change = (index: number, patch: Partial<BudgetDraft>) =>
		setDrafts(drafts.map((d, i) => (i === index ? { ...d, ...patch } : d)))
	const add = (value: string) => {
		if (value === '') return
		const agent = agents.find((a) => a.id === value)
		setDrafts([
			...drafts,
			value === 'organization'
				? { agentId: null, label: 'Organisation', limit: '', threshold: '80' }
				: {
						agentId: value,
						label: agent?.name ?? 'Agent',
						limit: '',
						threshold: '80',
					},
		])
	}

	return (
		<form
			aria-label="Modifier les budgets"
			onSubmit={(event) => {
				event.preventDefault()
				setInvalid(onSubmit(drafts))
			}}
		>
			<Rows>
				{drafts.map((draft, index) => (
					<Row key={draft.agentId ?? 'organization'}>
						<span className="min-w-0 flex-1 truncate font-medium">
							{draft.label}
						</span>
						<label className="flex items-center gap-1.5 text-xs text-muted-foreground">
							Limite ($)
							<input
								inputMode="decimal"
								aria-label={`Limite mensuelle en dollars : ${draft.label}`}
								value={draft.limit}
								onChange={(e) => change(index, { limit: e.target.value })}
								className={`${CTL} w-24`}
							/>
						</label>
						<label className="flex items-center gap-1.5 text-xs text-muted-foreground">
							Alerte (%)
							<input
								inputMode="numeric"
								aria-label={`Seuil d'alerte en pourcentage : ${draft.label}`}
								value={draft.threshold}
								onChange={(e) => change(index, { threshold: e.target.value })}
								className={`${CTL} w-16`}
							/>
						</label>
						<Button
							type="button"
							variant="ghost"
							size="icon"
							aria-label={`Retirer le budget : ${draft.label}`}
							onClick={() => setDrafts(drafts.filter((_, i) => i !== index))}
						>
							<Trash2 aria-hidden="true" />
						</Button>
					</Row>
				))}
			</Rows>
			<div className="flex flex-wrap items-center gap-2 border-t px-3 py-2">
				<Plus aria-hidden="true" className="size-4 text-muted-foreground" />
				<NativeSelect
					aria-label="Ajouter un budget"
					value=""
					onChange={(e) => add(e.target.value)}
					className="w-auto"
				>
					<option value="">Ajouter un budget…</option>
					{hasOrganization ? null : (
						<option value="organization">Organisation</option>
					)}
					{available.map((agent) => (
						<option key={agent.id} value={agent.id}>
							{agent.name}
						</option>
					))}
				</NativeSelect>
				<span className="flex-1" />
				<Button type="button" variant="outline" onClick={onCancel}>
					Annuler
				</Button>
				<Button type="submit" disabled={saving}>
					{saving ? 'Enregistrement…' : 'Enregistrer'}
				</Button>
			</div>
			{invalid ? (
				<div className="px-3 pb-3">
					<InlineAlert tone="danger">{invalid}</InlineAlert>
				</div>
			) : null}
		</form>
	)
}

export function BudgetsList({ budgets, agents, saving, error, onSave }: Props) {
	const [editing, setEditing] = useState(false)
	return (
		<>
			<div className="mb-2 flex items-center justify-end">
				{editing ? null : (
					<Button variant="outline" onClick={() => setEditing(true)}>
						Modifier les budgets
					</Button>
				)}
			</div>
			{error ? (
				<InlineAlert tone="danger" className="mb-2">
					{error}
				</InlineAlert>
			) : null}
			<Panel>
				{editing ? (
					<BudgetEditor
						initial={draftsOf(budgets.items)}
						agents={agents}
						saving={saving}
						onCancel={() => setEditing(false)}
						onSubmit={(drafts) => {
							const result = saveRequestOf(drafts)
							if (!result.ok) return result.reason
							onSave(result.request, () => setEditing(false))
							return null
						}}
					/>
				) : (
					<BudgetRows budgets={budgets} />
				)}
			</Panel>
		</>
	)
}
