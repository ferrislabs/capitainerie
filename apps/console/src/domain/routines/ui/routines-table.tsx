import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import { Button } from '#/components/ui/button'
import type { Routine } from '#/domain/routines/types'
import { AgentLabel } from '#/domain/routines/ui/agent-avatar'
import { RunBadge } from '#/domain/routines/ui/run-badge'
import { Sw, Tbl, Td, Th, Tr } from '#/domain/sources/ui/mockup'

export interface RoutineRow {
	routine: Routine
	scheduleLabel: string
}

interface Props {
	rows: ReadonlyArray<RoutineRow>
	onLaunch: (id: string) => void
	onToggle: (id: string) => void
}

export function RoutinesTable({ rows, onLaunch, onToggle }: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Routine</Th>
					<Th>Agent</Th>
					<Th>Fréquence</Th>
					<Th>Dernier passage</Th>
					<Th>
						<span className="sr-only">Lancer</span>
					</Th>
					<Th>Actif</Th>
				</tr>
			</thead>
			<tbody>
				{rows.map(({ routine, scheduleLabel }) => (
					<Tr key={routine.id}>
						<Td>
							<Link
								to="/routines/$routineId/{-$tab}"
								params={{ routineId: routine.id }}
								className="font-medium text-foreground"
							>
								{routine.name}
							</Link>
						</Td>
						<Td>
							<AgentLabel name={routine.agent} />
						</Td>
						<Td className="text-muted-foreground">{scheduleLabel}</Td>
						<Td>
							<span className="text-muted-foreground">
								{routine.lastRun.when}
							</span>{' '}
							<RunBadge
								tone={routine.lastRun.tone}
								label={routine.lastRun.label}
								listing
							/>
						</Td>
						<Td className="text-right">
							<Button
								type="button"
								size="sm"
								variant="outline"
								onClick={() => onLaunch(routine.id)}
							>
								<Play aria-hidden="true" />
								Lancer
							</Button>
						</Td>
						<Td>
							<Sw
								aria-label={`Activer ${routine.name}`}
								checked={routine.enabled}
								onCheckedChange={() => onToggle(routine.id)}
							/>
						</Td>
					</Tr>
				))}
			</tbody>
		</Tbl>
	)
}
