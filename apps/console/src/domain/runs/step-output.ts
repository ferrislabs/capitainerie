import type { Schemas } from '#/api/api.client'
import { stepDetailText } from '#/domain/runs/step-text'

export function stepOutputValue(
	step: Pick<Schemas.StepResponse, 'detail' | 'execution' | 'error'>,
): Record<string, unknown> | undefined {
	const output: Record<string, unknown> = {}
	if (step.detail) output.detail = stepDetailText(step.detail)
	if (step.execution) output.execution = step.execution
	if (step.error) output.error = step.error
	return Object.keys(output).length === 0 ? undefined : output
}
