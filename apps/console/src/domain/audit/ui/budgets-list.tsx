import { SectionCard } from '#/components/layout/section-card'
import type { Budget } from '#/domain/audit/mock/audit-data'

interface Props {
	budgets: ReadonlyArray<Budget>
}

const format = (value: number): string =>
	value.toLocaleString('fr-FR', { minimumFractionDigits: 2 })

export function BudgetsList({ budgets }: Props) {
	return (
		<SectionCard title="Budgets mensuels" flush>
			<ul className="divide-y">
				{budgets.map((budget) => {
					const percent = Math.min(
						100,
						Math.round((budget.spent / budget.limit) * 100),
					)
					return (
						<li
							key={budget.scope}
							className="flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-2"
						>
							<span className="min-w-0 flex-1">
								<b className="font-medium">{budget.scope}</b>
								<span className="block text-xs text-muted-foreground">
									{budget.kind === 'organisation' ? 'Organisation' : 'Agent'}
								</span>
							</span>
							<progress
								value={percent}
								max={100}
								aria-label={`Budget consommé : ${budget.scope}`}
								className="h-1.5 w-44 overflow-hidden rounded-full [&::-moz-progress-bar]:bg-primary [&::-webkit-progress-bar]:bg-muted [&::-webkit-progress-value]:bg-primary"
							/>
							<span className="figures w-32 text-right text-xs text-muted-foreground">
								{format(budget.spent)} $ / {budget.limit} $
							</span>
						</li>
					)
				})}
			</ul>
		</SectionCard>
	)
}
