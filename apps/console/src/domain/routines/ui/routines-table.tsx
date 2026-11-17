import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import { Button } from '#/components/ui/button'
import { Switch } from '#/components/ui/switch'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { Routine } from '#/domain/routines/types'
import { AgentLabel } from '#/domain/routines/ui/agent-avatar'
import { RunBadge } from '#/domain/routines/ui/run-badge'

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
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Routine</TableHead>
					<TableHead>Agent</TableHead>
					<TableHead>Fréquence</TableHead>
					<TableHead>Dernier passage</TableHead>
					<TableHead>
						<span className="sr-only">Lancer</span>
					</TableHead>
					<TableHead>Actif</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{rows.map(({ routine, scheduleLabel }) => (
					<TableRow key={routine.id}>
						<TableCell className="font-medium">
							<Link
								to="/routines/$routineId/{-$tab}"
								params={{ routineId: routine.id }}
								className="hover:underline"
							>
								{routine.name}
							</Link>
						</TableCell>
						<TableCell>
							<AgentLabel name={routine.agent} />
						</TableCell>
						<TableCell className="text-muted-foreground">
							{scheduleLabel}
						</TableCell>
						<TableCell>
							<span className="mr-2 text-muted-foreground">
								{routine.lastRun.when}
							</span>
							<RunBadge
								tone={routine.lastRun.tone}
								label={routine.lastRun.label}
							/>
						</TableCell>
						<TableCell className="text-right">
							<Button
								type="button"
								size="sm"
								variant="outline"
								onClick={() => onLaunch(routine.id)}
							>
								<Play aria-hidden="true" />
								Lancer
							</Button>
						</TableCell>
						<TableCell>
							<Switch
								aria-label={`Activer ${routine.name}`}
								checked={routine.enabled}
								onCheckedChange={() => onToggle(routine.id)}
							/>
						</TableCell>
					</TableRow>
				))}
			</TableBody>
		</Table>
	)
}
