import { useState } from 'react'
import { Button } from '#/components/ui/button'
import {
	CRITERIA,
	type CriterionId,
	type CriterionResult,
	type Dossier,
	type OnFail,
	type PipelineCriterion,
} from '#/domain/sources/criteria'
import {
	AddRow,
	CTL,
	Sev,
	Sw,
	Tbl,
	Td,
	Th,
	Tr,
} from '#/domain/sources/ui/mockup'
import { ResultBadge } from '#/domain/sources/ui/result-badge'
import { cn } from '#/lib/utils'

const ON_FAIL: ReadonlyArray<{ value: OnFail; label: string }> = [
	{ value: 'reject', label: 'Rejeter' },
	{ value: 'review', label: 'À valider' },
	{ value: 'ignore', label: 'Ignorer' },
]

interface Props {
	criteria: ReadonlyArray<PipelineCriterion>
	dossier: Dossier
	available: ReadonlyArray<CriterionId>
	presets: ReadonlyArray<string>
	onToggle: (index: number) => void
	onParam: (index: number, param: string) => void
	onOnFail: (index: number, onFail: OnFail) => void
	onRemove: (index: number) => void
	onAdd: (id: CriterionId) => void
	onPreset: (name: string) => void
}

export function PipelineCriteria({
	criteria,
	dossier,
	available,
	presets,
	onToggle,
	onParam,
	onOnFail,
	onRemove,
	onAdd,
	onPreset,
}: Props) {
	const [picked, setPicked] = useState<CriterionId | ''>('')
	const choice = available.includes(picked as CriterionId)
		? (picked as CriterionId)
		: available[0]
	return (
		<div>
			<Tbl>
				<thead>
					<tr>
						<Th>Actif</Th>
						<Th>Critère</Th>
						<Th>Valeur</Th>
						<Th>Si non</Th>
						<Th>Dossier</Th>
						<Th />
					</tr>
				</thead>
				<tbody>
					{criteria.length === 0 ? (
						<tr>
							<Td colSpan={6} className="p-3.5 text-muted-foreground">
								Aucun critère : tout dossier est qualifié.
							</Td>
						</tr>
					) : null}
					{criteria.map((c, index) => {
						const def = CRITERIA[c.id]
						const result: CriterionResult = def.evaluate(dossier, c.param)
						return (
							<Tr key={c.id}>
								<Td>
									<Sw
										aria-label={`Activer ${def.label}`}
										checked={c.enabled}
										onCheckedChange={() => onToggle(index)}
									/>
								</Td>
								<Td>{def.label}</Td>
								<Td>
									{def.defaultParam !== undefined ? (
										<>
											<input
												className={cn(CTL, 'w-[130px] font-mono text-[11.5px]')}
												aria-label={`Valeur : ${def.label}`}
												value={String(c.param ?? '')}
												onChange={(e) => onParam(index, e.target.value)}
											/>
											{def.unit ? (
												<span className="text-muted-foreground">
													{' '}
													{def.unit}
												</span>
											) : null}
										</>
									) : (
										<span className="text-muted-foreground">—</span>
									)}
								</Td>
								<Td>
									<select
										className={cn(CTL, 'w-auto')}
										aria-label={`Si non : ${def.label}`}
										value={c.onFail}
										onChange={(e) => onOnFail(index, e.target.value as OnFail)}
									>
										{ON_FAIL.map((o) => (
											<option key={o.value} value={o.value}>
												{o.label}
											</option>
										))}
									</select>
								</Td>
								<Td>
									{c.enabled ? (
										<ResultBadge result={result} />
									) : (
										<Sev tone="info">Désactivé</Sev>
									)}
								</Td>
								<Td className="text-right">
									<Button
										type="button"
										size="sm"
										variant="outline"
										aria-label={`Retirer ${def.label}`}
										onClick={() => onRemove(index)}
									>
										Retirer
									</Button>
								</Td>
							</Tr>
						)
					})}
				</tbody>
			</Tbl>
			<AddRow className="pt-3">
				{choice ? (
					<>
						<select
							className={cn(CTL, 'flex-1')}
							aria-label="Critère à ajouter"
							value={choice}
							onChange={(e) => setPicked(e.target.value as CriterionId)}
						>
							{available.map((id) => (
								<option key={id} value={id}>
									{CRITERIA[id].label}
								</option>
							))}
						</select>
						<Button
							type="button"
							variant="outline"
							onClick={() => onAdd(choice)}
						>
							Ajouter un critère
						</Button>
					</>
				) : (
					<span className="text-muted-foreground">
						Tous les critères sont utilisés.
					</span>
				)}
			</AddRow>
			<AddRow className="flex-wrap items-center">
				<span className="text-muted-foreground">Préréglages :</span>
				{presets.map((name) => (
					<Button
						key={name}
						type="button"
						size="sm"
						variant="outline"
						onClick={() => onPreset(name)}
					>
						{name}
					</Button>
				))}
			</AddRow>
		</div>
	)
}
