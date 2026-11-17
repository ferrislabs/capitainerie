import { useState } from 'react'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
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
import { TextInput } from '#/components/ui/text-input'
import {
	CRITERIA,
	type CriterionId,
	type CriterionResult,
	type Dossier,
	type OnFail,
	type PipelineCriterion,
} from '#/domain/sources/criteria'
import { ResultBadge } from '#/domain/sources/ui/result-badge'

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
		<div className="flex flex-col gap-3">
			<Table>
				<TableHeader>
					<TableRow>
						<TableHead>Actif</TableHead>
						<TableHead>Critère</TableHead>
						<TableHead>Valeur</TableHead>
						<TableHead>Si non</TableHead>
						<TableHead>Dossier</TableHead>
						<TableHead />
					</TableRow>
				</TableHeader>
				<TableBody>
					{criteria.length === 0 ? (
						<TableRow>
							<TableCell colSpan={6} className="text-muted-foreground">
								Aucun critère : tout dossier est qualifié.
							</TableCell>
						</TableRow>
					) : null}
					{criteria.map((c, index) => {
						const def = CRITERIA[c.id]
						const result: CriterionResult = def.evaluate(dossier, c.param)
						return (
							<TableRow key={c.id}>
								<TableCell>
									<Switch
										aria-label={`Activer ${def.label}`}
										checked={c.enabled}
										onCheckedChange={() => onToggle(index)}
									/>
								</TableCell>
								<TableCell>{def.label}</TableCell>
								<TableCell>
									{def.defaultParam !== undefined ? (
										<span className="flex items-center gap-1">
											<TextInput
												className="w-32 font-mono"
												aria-label={`Valeur : ${def.label}`}
												value={String(c.param ?? '')}
												onChange={(e) => onParam(index, e.target.value)}
											/>
											{def.unit ? (
												<span className="text-muted-foreground">
													{def.unit}
												</span>
											) : null}
										</span>
									) : (
										<span className="text-muted-foreground">—</span>
									)}
								</TableCell>
								<TableCell>
									<NativeSelect
										aria-label={`Si non : ${def.label}`}
										value={c.onFail}
										onChange={(e) => onOnFail(index, e.target.value as OnFail)}
									>
										{ON_FAIL.map((o) => (
											<option key={o.value} value={o.value}>
												{o.label}
											</option>
										))}
									</NativeSelect>
								</TableCell>
								<TableCell>
									{c.enabled ? (
										<ResultBadge result={result} />
									) : (
										<Badge variant="neutral">Désactivé</Badge>
									)}
								</TableCell>
								<TableCell className="text-right">
									<Button
										type="button"
										size="sm"
										variant="outline"
										aria-label={`Retirer ${def.label}`}
										onClick={() => onRemove(index)}
									>
										Retirer
									</Button>
								</TableCell>
							</TableRow>
						)
					})}
				</TableBody>
			</Table>
			<div className="flex flex-wrap items-center gap-2 px-3">
				{choice ? (
					<>
						<NativeSelect
							aria-label="Critère à ajouter"
							value={choice}
							onChange={(e) => setPicked(e.target.value as CriterionId)}
						>
							{available.map((id) => (
								<option key={id} value={id}>
									{CRITERIA[id].label}
								</option>
							))}
						</NativeSelect>
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
			</div>
			<div className="flex flex-wrap items-center gap-2 px-3 pb-3">
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
			</div>
		</div>
	)
}
