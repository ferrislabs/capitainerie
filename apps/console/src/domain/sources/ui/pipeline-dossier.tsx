import { Button } from '#/components/ui/button'
import type { Decision, Dossier } from '#/domain/sources/criteria'
import { CRITERIA } from '#/domain/sources/criteria'
import { KeyValueRows } from '#/domain/sources/ui/key-value-rows'
import { Sev } from '#/domain/sources/ui/mockup'
import { resultLabel } from '#/domain/sources/ui/result-badge'
import { cn } from '#/lib/utils'

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
			<fieldset className="m-0 min-w-0 border-0 p-0">
				<legend className="sr-only">Dossier</legend>
				<div className="mx-3 my-2.5 inline-flex flex-wrap overflow-hidden rounded-md border">
					{segments.map((segment) => (
						<Button
							key={segment.value}
							type="button"
							variant="ghost"
							aria-pressed={selected === segment.value}
							onClick={() => onSelect(segment.value)}
							className={cn(
								'rounded-none bg-card px-2.5 font-normal text-muted-foreground hover:bg-secondary hover:text-foreground',
								selected === segment.value && 'bg-secondary text-foreground',
							)}
						>
							{segment.label}
						</Button>
					))}
				</div>
			</fieldset>
			<KeyValueRows
				rows={[
					['Raison sociale', dossier.raison],
					[
						'SIREN',
						<span key="siren">
							{dossier.siren}{' '}
							<span className="text-muted-foreground">
								{`· ${plural(dossier.indices, 'indice')} ${
									dossier.indices > 1 ? 'concordants' : 'concordant'
								}`}
							</span>
						</span>,
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

const VERDICT = {
	qualified: { label: 'Qualifié', tone: 'ok' },
	review: { label: 'À valider', tone: 'warn' },
	rejected: { label: 'Rejeté', tone: 'crit' },
} as const

export function VerdictPanel({ decision }: { decision: Decision }) {
	const reasons = decision.rows.filter(
		(r) => r.result !== 'ok' && r.criterion.onFail !== 'ignore',
	)
	const verdict = VERDICT[decision.verdict]
	return (
		<div className="px-3 py-2.5">
			<Sev tone={verdict.tone} className="px-3 py-[3px] text-[13px]">
				{verdict.label}
			</Sev>{' '}
			<span className="text-muted-foreground">
				{VERDICT_TEXT[decision.verdict]}
			</span>
			{reasons.map((r) => (
				<div key={r.criterion.id} className="mt-1 text-muted-foreground">
					· {CRITERIA[r.criterion.id].label} :{' '}
					{resultLabel(r.result).toLowerCase()}
				</div>
			))}
		</div>
	)
}
