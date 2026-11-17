import type { Budget } from '#/domain/audit/mock/audit-data'
import { Panel, Row, Rows, RowTitle } from '#/domain/audit/ui/rows'

interface Props {
	budgets: ReadonlyArray<Budget>
}

const format = (value: number): string =>
	value.toLocaleString('fr-FR', { minimumFractionDigits: 2 })

export function BudgetsList({ budgets }: Props) {
	return (
		<Panel>
			<Rows>
				{budgets.map((budget) => {
					const percent = Math.min(
						100,
						Math.round((budget.spent / budget.limit) * 100),
					)
					return (
						<Row key={budget.scope}>
							<RowTitle
								title={budget.scope}
								sub={`${format(budget.spent)} $ sur ${budget.limit} $ ce mois`}
							/>
							<div
								role="progressbar"
								aria-label={`Budget consommé : ${budget.scope}`}
								aria-valuemin={0}
								aria-valuemax={100}
								aria-valuenow={percent}
								className="h-1.5 w-[180px] min-w-[60px] overflow-hidden rounded-[3px] bg-secondary"
							>
								<i
									className="block h-full rounded-[3px] bg-primary"
									style={{ width: `${Math.max(percent, 1)}%` }}
								/>
							</div>
							<span className="figures w-9 text-right font-mono text-[11.5px] text-muted-foreground">
								{percent} %
							</span>
						</Row>
					)
				})}
			</Rows>
		</Panel>
	)
}
