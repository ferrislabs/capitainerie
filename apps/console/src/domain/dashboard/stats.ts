import type { DemoAgent, DemoApproval, DemoTask } from '#/domain/demo'

export interface DashboardStats {
	running: number
	open: number
	blocked: number
	pending: number
}

const isOpen = (t: DemoTask) => t.status !== 'done' && t.status !== 'cancelled'

export function dashboardStats(
	agents: ReadonlyArray<DemoAgent>,
	tasks: ReadonlyArray<DemoTask>,
	approvals: ReadonlyArray<DemoApproval>,
): DashboardStats {
	return {
		running: agents.filter((a) => a.status === 'running').length,
		open: tasks.filter(isOpen).length,
		blocked: tasks.filter((t) => t.status === 'blocked').length,
		pending: approvals.filter((a) => a.status === 'pending').length,
	}
}

export function currentTask(
	tasks: ReadonlyArray<DemoTask>,
	agentId: string,
): DemoTask | undefined {
	return tasks.find((t) => t.agentId === agentId && isOpen(t))
}
