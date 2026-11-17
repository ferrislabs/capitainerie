import { Button } from '#/components/ui/button'
import type { Decision, Dossier } from '#/domain/sources/criteria'
import { CRITERIA } from '#/domain/sources/criteria'
import { KeyValueRows } from '#/domain/sources/ui/key-value-rows'
import { resultLabel, VerdictBadge } from '#/domain/sources/ui/result-badge'

interface Segment {
	value: string
	label: string
}

interface DossierProps {
	dossier: Dossier
	segments: ReadonlyArray<Segment>
	selected: string
	onSelect: (value: string) => void
}

const plural = (n: number, word: string) => `${n} ${word}${n > 1 ? 's' : ''}`

export function DossierPanel({
	dossier,
	segments,
	selected,
	onSelect,
}: DossierProps) {
	return (
		<div>
			<fieldset className="m-0 flex min-w-0 flex-wrap gap-1 border-0 p-3">
				<legend className="sr-only">Dossier</legend>
				{segments.map((segment) => (
					<Button
						key={segment.value}
						type="button"
						size="sm"
						variant={selected === segment.value ? 'default' : 'outline'}
						aria-pressed={selected === segment.value}
						onClick={() => onSelect(segment.value)}
					>
						{segment.label}
					</Button>
				))}
			</fieldset>
			<KeyValueRows
				rows={[
					['Raison sociale', dossier.raison],
					[
						'SIREN',
						`${dossier.siren} · ${plural(dossier.indices, 'indice')} ${
							dossier.indices > 1 ? 'concordants' : 'concordant'
						}`,
					],
					['Statut', dossier.active ? 'Active' : 'Radiée'],
					['Procédure collective', dossier.collective ? 'Oui (BODACC)' : 'Non'],
					[
						'Effectif · NAF',
						`${dossier.effectif === null ? 'inconnu' : dossier.effectif} · ${dossier.naf}`,
					],
					['Pays · domaine', `${dossier.pays} · ${dossier.age} jours`],
					['Titulaire RDAP', dossier.rdapMasque ? 'Masqué' : 'Visible'],
					['Déjà dans le CRM', dossier.crm ? 'Oui' : 'Non'],
				]}
			/>
		</div>
	)
}

const VERDICT_TEXT = {
	qualified: 'Tous les critères actifs sont remplis : export CRM.',
	review:
		'Un critère est incertain ou à valider : une personne tranche dans Approbations.',
	rejected: "Un critère bloquant n'est pas rempli : aucun export.",
} as const

export function VerdictPanel({ decision }: { decision: Decision }) {
	const reasons = decision.rows.filter(
		(r) => r.result !== 'ok' && r.criterion.onFail !== 'ignore',
	)
	return (
		<div className="flex flex-col gap-1 p-3">
			<p className="flex flex-wrap items-center gap-2">
				<VerdictBadge verdict={decision.verdict} />
				<span className="text-muted-foreground">
					{VERDICT_TEXT[decision.verdict]}
				</span>
			</p>
			{reasons.map((r) => (
				<p key={r.criterion.id} className="text-muted-foreground">
					· {CRITERIA[r.criterion.id].label} :{' '}
					{resultLabel(r.result).toLowerCase()}
				</p>
			))}
		</div>
	)
}
