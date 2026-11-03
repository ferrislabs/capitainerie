import { describe, expect, it } from 'vitest'
import {
	reasonLabel,
	targetSummary,
	toolLabel,
} from '#/domain/integrations/journal/action-labels'

describe('toolLabel', () => {
	it('humanizes the Odoo and n8n tools', () => {
		expect(toolLabel('odoo_write')).toBe('Odoo · modifier')
		expect(toolLabel('odoo_search_read')).toBe('Odoo · rechercher')
		expect(toolLabel('odoo_call_method')).toBe('Odoo · appeler une méthode')
		expect(toolLabel('n8n_update_workflow')).toBe('n8n · modifier un workflow')
		expect(toolLabel('n8n_list_executions')).toBe('n8n · lister les exécutions')
	})

	it('falls back to the raw name', () => {
		expect(toolLabel('odoo_teleport')).toBe('odoo_teleport')
		expect(toolLabel('constructor')).toBe('constructor')
	})
})

describe('targetSummary', () => {
	it('names the Odoo model and the ids it touches', () => {
		expect(targetSummary({ model: 'res.partner', ids: [2, 1] })).toBe(
			'res.partner #2, #1',
		)
		expect(targetSummary({ model: 'res.partner' })).toBe('res.partner')
		expect(
			targetSummary({
				model: 'sale.order',
				method: 'action_confirm',
				ids: [8],
			}),
		).toBe('sale.order · action_confirm #8')
		expect(targetSummary({ attachment_id: 7 })).toBe('pièce jointe #7')
	})

	it('shortens a long list of ids', () => {
		expect(
			targetSummary({ model: 'res.partner', ids: [1, 2, 3, 4, 5, 6, 7] }),
		).toBe('res.partner #1, #2, #3, #4, #5 et 2 autres')
	})

	it('names the n8n workflow, execution or tag', () => {
		expect(targetSummary({ workflow_id: 'wf1' })).toBe('workflow wf1')
		expect(targetSummary({ workflow_name: 'Relance' })).toBe(
			'workflow « Relance »',
		)
		expect(targetSummary({ execution_id: '42' })).toBe('exécution 42')
		expect(targetSummary({ tag_name: 'urgent' })).toBe('étiquette « urgent »')
		expect(targetSummary({ tag_id: 't1' })).toBe('étiquette t1')
		expect(targetSummary({ resource: 'workflows' })).toBe('workflows')
		expect(targetSummary({ resource: 'executions', workflow_id: 'wf1' })).toBe(
			'exécutions · workflow wf1',
		)
	})

	it('has nothing to say about an empty or unknown target', () => {
		expect(targetSummary(null)).toBe('—')
		expect(targetSummary(undefined)).toBe('—')
		expect(targetSummary({})).toBe('—')
		expect(targetSummary('x')).toBe('—')
		expect(targetSummary({ odd: true })).toBe('{"odd":true}')
	})
})

describe('reasonLabel', () => {
	it('translates the known codes', () => {
		const known: Array<[string, string]> = [
			[
				'instance_not_granted',
				'instance non autorisée pour cette intervention',
			],
			['write_not_granted', 'écriture non autorisée sur cette instance'],
			[
				'instance_of_another_client',
				'instance d’un autre client que celui de la carte',
			],
			['instance_archived', 'instance archivée'],
			['sensitive_model', 'modèle Odoo sensible, inaccessible'],
			['policy_denied', 'refusé par la politique d’accès'],
			[
				'masked_value_without_original',
				'valeur masquée dont l’original n’est pas connu',
			],
			['not_supported', 'non pris en charge par cette instance'],
			[
				'before_unreadable',
				'l’état d’avant n’a pas pu être lu : rien n’a été écrit',
			],
			['connection_failed', 'connexion à l’instance impossible'],
		]
		for (const [code, label] of known) {
			expect(reasonLabel(code, 'raw english')).toBe(label)
		}
	})

	it('falls back to the raw reason, then the code', () => {
		expect(reasonLabel('something_new', 'it broke')).toBe('it broke')
		expect(reasonLabel('something_new', null)).toBe('something_new')
		expect(reasonLabel(null, 'it broke')).toBe('it broke')
		expect(reasonLabel(null, null)).toBeUndefined()
		expect(reasonLabel(undefined, '')).toBeUndefined()
	})
})
