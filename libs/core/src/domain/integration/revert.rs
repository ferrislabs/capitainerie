use std::collections::BTreeSet;

use capitainerie_common::CoreError;
use serde_json::{Map, Value, json};

use super::{
    ConnectorFailure,
    action::{ActionRequest, N8nId, ODOO_MAX_DELETE_IDS, OdooModel},
    journal::{IntegrationAction, Outcome},
    redact::redact,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RevertRefusal {
    #[error("this action cannot be restored")]
    NotRevertible,
    #[error("this action is already restored")]
    AlreadyReverted,
    #[error("the intervention is still running: wait for it to end")]
    InterventionRunning,
    #[error("the instance is archived")]
    InstanceArchived,
    #[error("restoring on a production instance needs confirm_production")]
    ProductionWriteNotConfirmed,
    #[error("the state changed since this action: nothing was restored")]
    ChangedSince,
}

impl RevertRefusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::NotRevertible => "not_revertible",
            Self::AlreadyReverted => "already_reverted",
            Self::InterventionRunning => "intervention_running",
            Self::InstanceArchived => "instance_archived",
            Self::ProductionWriteNotConfirmed => "production_write_not_confirmed",
            Self::ChangedSince => "changed_since",
        }
    }

    pub fn is_conflict(self) -> bool {
        matches!(
            self,
            Self::AlreadyReverted | Self::InterventionRunning | Self::ChangedSince
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RevertError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Refused(#[from] RevertRefusal),
    #[error(transparent)]
    Connector(#[from] ConnectorFailure),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevertPlan {
    pub check: Option<ActionRequest>,
    pub writes: Vec<ActionRequest>,
}

pub fn ensure_revertible(action: &IntegrationAction) -> Result<(), RevertRefusal> {
    if action.outcome == Outcome::Succeeded
        && action.revertible
        && action.reverts_action_id.is_none()
    {
        Ok(())
    } else {
        Err(RevertRefusal::NotRevertible)
    }
}

pub fn plan(
    action: &IntegrationAction,
    snapshot: Option<&Value>,
    server_version: Option<&str>,
) -> Result<RevertPlan, RevertRefusal> {
    ensure_revertible(action)?;
    let inverse = match action.tool.as_str() {
        "odoo_write" => undo_odoo_write(action, snapshot),
        "odoo_create" => undo_odoo_create(action),
        "n8n_update_workflow" => undo_n8n_update(action, snapshot),
        "n8n_publish_workflow" => undo_n8n_switch(action, snapshot, true),
        "n8n_unpublish_workflow" => undo_n8n_switch(action, snapshot, false),
        "n8n_archive_workflow" => undo_n8n_archive(action),
        "n8n_set_workflow_tags" => undo_n8n_tags(action, snapshot),
        "n8n_create_tag" => undo_n8n_create_tag(action),
        "n8n_create_workflow" => undo_n8n_create_workflow(action, server_version),
        _ => None,
    };
    inverse
        .filter(|inverse| !inverse.writes.is_empty())
        .ok_or(RevertRefusal::NotRevertible)
}

pub fn unchanged(action: &IntegrationAction, current: &Value) -> bool {
    match action.tool.as_str() {
        "odoo_write" | "odoo_create" => odoo_unchanged(action, current),
        "n8n_update_workflow" | "n8n_create_workflow" => action
            .after
            .as_ref()
            .is_some_and(|after| same_stamps(after, current)),
        "n8n_publish_workflow" | "n8n_unpublish_workflow" => {
            let state = published(current);
            state.is_some() && state == action.after.as_ref().and_then(published)
        }
        "n8n_archive_workflow" => current.get("isArchived").and_then(Value::as_bool) == Some(true),
        "n8n_set_workflow_tags" => {
            let after = action.after.as_ref().and_then(tag_set);
            after.is_some() && after == tag_set(current)
        }
        "n8n_create_tag" => true,
        _ => false,
    }
}

fn text_of<'a>(action: &'a IntegrationAction, key: &str) -> Option<&'a str> {
    [&action.target, &action.arguments]
        .into_iter()
        .find_map(|source| source.get(key)?.as_str())
}

fn odoo_model(action: &IntegrationAction) -> Option<OdooModel> {
    OdooModel::try_from(text_of(action, "model")?).ok()
}

fn workflow_id(action: &IntegrationAction) -> Option<N8nId> {
    N8nId::try_from(text_of(action, "workflow_id")?).ok()
}

fn ids_of(value: &Value) -> Option<Vec<i64>> {
    value.as_array()?.iter().map(Value::as_i64).collect()
}

fn written_fields(action: &IntegrationAction) -> Option<Vec<String>> {
    let values = action.arguments.get("values")?.as_object()?;
    Some(values.keys().cloned().collect())
}

fn record_ids(records: &Value) -> Option<Vec<i64>> {
    records
        .as_array()?
        .iter()
        .map(|record| record.get("id")?.as_i64())
        .collect()
}

fn is_integer(value: &Value) -> bool {
    value.is_i64() || value.is_u64()
}

fn writable(read: &Value) -> Value {
    match read.as_array().map(Vec::as_slice) {
        Some([id, Value::String(_)]) if is_integer(id) => id.clone(),
        Some(items) if items.iter().all(is_integer) => json!([[6, 0, items]]),
        _ => read.clone(),
    }
}

fn undo_odoo_write(action: &IntegrationAction, snapshot: Option<&Value>) -> Option<RevertPlan> {
    let model = odoo_model(action)?;
    let fields = written_fields(action)?;
    let ids = action
        .arguments
        .get("ids")
        .or_else(|| action.target.get("ids"))
        .and_then(ids_of)?;
    action.after.as_ref()?.as_array()?;
    let mut writes = Vec::new();
    for record in snapshot?.as_array()? {
        let record = record.as_object()?;
        let id = record.get("id")?.as_i64()?;
        let values: Map<String, Value> = record
            .iter()
            .filter(|(key, _)| key.as_str() != "id")
            .map(|(key, value)| (key.clone(), writable(value)))
            .collect();
        if !values.is_empty() {
            writes.push(ActionRequest::OdooWrite {
                model: model.clone(),
                ids: vec![id],
                values,
            });
        }
    }
    Some(RevertPlan {
        check: Some(ActionRequest::OdooRead { model, ids, fields }),
        writes,
    })
}

fn undo_odoo_create(action: &IntegrationAction) -> Option<RevertPlan> {
    let model = odoo_model(action)?;
    let ids = record_ids(action.after.as_ref()?)?;
    if ids.is_empty() || ids.len() > ODOO_MAX_DELETE_IDS {
        return None;
    }
    Some(RevertPlan {
        check: Some(ActionRequest::OdooRead {
            model: model.clone(),
            ids: ids.clone(),
            fields: Vec::new(),
        }),
        writes: vec![ActionRequest::OdooDelete { model, ids }],
    })
}

fn check_workflow(workflow_id: &N8nId) -> ActionRequest {
    ActionRequest::N8nGetWorkflow {
        workflow_id: workflow_id.clone(),
    }
}

fn undo_n8n_update(action: &IntegrationAction, snapshot: Option<&Value>) -> Option<RevertPlan> {
    let workflow_id = workflow_id(action)?;
    action.after.as_ref()?;
    let workflow = snapshot?.as_object()?;
    Some(RevertPlan {
        check: Some(check_workflow(&workflow_id)),
        writes: vec![ActionRequest::N8nUpdateWorkflow {
            workflow_id,
            name: Some(workflow.get("name")?.as_str()?.to_owned()),
            nodes: workflow.get("nodes")?.as_array()?.clone(),
            connections: workflow.get("connections")?.as_object()?.clone(),
            settings: Some(
                workflow
                    .get("settings")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default(),
            ),
            publish: workflow.get("active").and_then(Value::as_bool) == Some(true),
        }],
    })
}

fn undo_n8n_switch(
    action: &IntegrationAction,
    snapshot: Option<&Value>,
    published_by_action: bool,
) -> Option<RevertPlan> {
    let workflow_id = workflow_id(action)?;
    action.after.as_ref()?;
    if snapshot.and_then(published) == Some(published_by_action) {
        return None;
    }
    let inverse = if published_by_action {
        ActionRequest::N8nUnpublishWorkflow {
            workflow_id: workflow_id.clone(),
        }
    } else {
        ActionRequest::N8nPublishWorkflow {
            workflow_id: workflow_id.clone(),
        }
    };
    Some(RevertPlan {
        check: Some(check_workflow(&workflow_id)),
        writes: vec![inverse],
    })
}

fn undo_n8n_archive(action: &IntegrationAction) -> Option<RevertPlan> {
    let workflow_id = workflow_id(action)?;
    Some(RevertPlan {
        check: Some(check_workflow(&workflow_id)),
        writes: vec![ActionRequest::N8nUnarchiveWorkflow { workflow_id }],
    })
}

fn undo_n8n_tags(action: &IntegrationAction, snapshot: Option<&Value>) -> Option<RevertPlan> {
    let workflow_id = workflow_id(action)?;
    action.after.as_ref().and_then(tag_list)?;
    let tag_ids = tag_ids(snapshot?)?;
    Some(RevertPlan {
        check: Some(ActionRequest::N8nGetWorkflowTags {
            workflow_id: workflow_id.clone(),
        }),
        writes: vec![ActionRequest::N8nSetWorkflowTags {
            workflow_id,
            tag_ids,
        }],
    })
}

fn undo_n8n_create_tag(action: &IntegrationAction) -> Option<RevertPlan> {
    let tag_id = n8n_id_of(action.after.as_ref()?.get("id")?)?;
    Some(RevertPlan {
        check: None,
        writes: vec![ActionRequest::N8nDeleteTag { tag_id }],
    })
}

fn undo_n8n_create_workflow(
    action: &IntegrationAction,
    server_version: Option<&str>,
) -> Option<RevertPlan> {
    if server_version != Some("2") {
        return None;
    }
    let workflow_id = n8n_id_of(action.after.as_ref()?.get("id")?)?;
    Some(RevertPlan {
        check: Some(check_workflow(&workflow_id)),
        writes: vec![ActionRequest::N8nArchiveWorkflow { workflow_id }],
    })
}

fn n8n_id_of(value: &Value) -> Option<N8nId> {
    match value {
        Value::String(text) => N8nId::try_from(text.as_str()).ok(),
        Value::Number(number) if is_integer(value) => {
            N8nId::try_from(number.to_string().as_str()).ok()
        }
        _ => None,
    }
}

fn tag_list(value: &Value) -> Option<&Vec<Value>> {
    value.as_array().or_else(|| value.get("tags")?.as_array())
}

fn tag_ids(value: &Value) -> Option<Vec<N8nId>> {
    tag_list(value)?
        .iter()
        .map(|tag| n8n_id_of(tag.get("id").unwrap_or(tag)))
        .collect()
}

fn tag_set(value: &Value) -> Option<BTreeSet<String>> {
    Some(
        tag_ids(value)?
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect(),
    )
}

fn published(workflow: &Value) -> Option<bool> {
    workflow.get("active").and_then(Value::as_bool).or_else(|| {
        workflow
            .get("activeVersionId")
            .map(|version| !version.is_null())
    })
}

fn same_stamps(after: &Value, current: &Value) -> bool {
    let stamp =
        |workflow: &Value, key: &str| workflow.get(key).filter(|value| !value.is_null()).cloned();
    let updated = stamp(after, "updatedAt");
    if updated.is_none() || updated != stamp(current, "updatedAt") {
        return false;
    }
    match (stamp(after, "versionId"), stamp(current, "versionId")) {
        (Some(left), Some(right)) => left == right,
        _ => true,
    }
}

fn odoo_unchanged(action: &IntegrationAction, current: &Value) -> bool {
    let (Some(fields), Some(after)) = (
        written_fields(action),
        action.after.as_ref().and_then(Value::as_array),
    ) else {
        return false;
    };
    let current = redact(current);
    let Some(current) = current.as_array() else {
        return false;
    };
    !after.is_empty()
        && after.iter().all(|expected| {
            let Some(id) = expected.get("id") else {
                return false;
            };
            current
                .iter()
                .find(|actual| actual.get("id") == Some(id))
                .is_some_and(|actual| {
                    fields
                        .iter()
                        .all(|field| actual.get(field) == expected.get(field))
                })
        })
}

#[cfg(test)]
mod tests {
    use capitainerie_common::{IntegrationActionId, IntegrationInstanceId, OrganizationId};
    use chrono::Utc;
    use serde_json::{Map, json};

    use super::*;
    use crate::domain::integration::{
        action::{Access, N8nId, OdooModel},
        journal::{INTEGRATOR_AGENT, Outcome},
        redact::MASK,
    };

    fn action(
        tool: &str,
        target: Value,
        arguments: Value,
        after: Option<Value>,
    ) -> IntegrationAction {
        IntegrationAction {
            id: IntegrationActionId::new(),
            organization_id: OrganizationId::new(),
            run_id: None,
            card_id: None,
            instance_id: IntegrationInstanceId::new(),
            subject: "user-1".into(),
            actor: INTEGRATOR_AGENT.into(),
            tool: tool.into(),
            access: Access::Write,
            target,
            arguments,
            outcome: Outcome::Succeeded,
            reason_code: None,
            reason: None,
            before: None,
            after,
            revertible: true,
            reverts_action_id: None,
            reverted_by_action_id: None,
            latency_ms: Some(3),
            created_at: Utc::now(),
            finished_at: Some(Utc::now()),
        }
    }

    fn model(name: &str) -> OdooModel {
        OdooModel::try_from(name).unwrap()
    }

    fn id(raw: &str) -> N8nId {
        N8nId::try_from(raw).unwrap()
    }

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn odoo_write(values: Value, after: Option<Value>) -> IntegrationAction {
        action(
            "odoo_write",
            json!({"model": "res.partner", "ids": [7, 8]}),
            json!({"model": "res.partner", "ids": [7, 8], "values": values}),
            after,
        )
    }

    fn odoo_create(after: Option<Value>) -> IntegrationAction {
        action(
            "odoo_create",
            json!({"model": "res.partner"}),
            json!({"model": "res.partner", "values": {"name": "Neuf", "email": "n@acme.test"}}),
            after,
        )
    }

    fn workflow_action(tool: &str, after: Option<Value>) -> IntegrationAction {
        action(
            tool,
            json!({"workflow_id": "wf1"}),
            json!({"workflow_id": "wf1"}),
            after,
        )
    }

    fn planned(
        action: &IntegrationAction,
        snapshot: Option<&Value>,
        version: Option<&str>,
    ) -> RevertPlan {
        plan(action, snapshot, version).unwrap()
    }

    fn sorted(mut fields: Vec<String>) -> Vec<String> {
        fields.sort();
        fields
    }

    fn workflow(active: bool) -> Value {
        json!({
            "id": "wf1",
            "name": "Relance factures",
            "active": active,
            "nodes": [{"name": "Start", "type": "n8n-nodes-base.manualTrigger"}],
            "connections": {"Start": {"main": []}},
            "settings": {"executionOrder": "v1"},
            "updatedAt": "2026-10-01T10:00:00.000Z",
            "versionId": "v-before"
        })
    }

    #[test]
    fn an_odoo_write_puts_the_old_values_back_record_by_record() {
        let written = json!({
            "name": "Acme SA",
            "parent_id": 9,
            "category_id": [[6, 0, [4]]],
            "comment": "new"
        });
        let snapshot = json!([
            {"id": 7, "name": "Acme", "parent_id": [3, "Boss"], "category_id": [1, 2], "comment": false},
            {"id": 8, "name": "Beta", "parent_id": false, "category_id": [], "comment": "note"}
        ]);

        let inverse = planned(
            &odoo_write(written, Some(json!([]))),
            Some(&snapshot),
            Some("19.0"),
        );

        let Some(ActionRequest::OdooRead {
            model: read,
            ids,
            fields,
        }) = inverse.check
        else {
            panic!("the check is a read")
        };
        assert_eq!(read, model("res.partner"));
        assert_eq!(ids, [7, 8]);
        assert_eq!(
            sorted(fields),
            ["category_id", "comment", "name", "parent_id"]
        );
        assert_eq!(
            inverse.writes,
            [
                ActionRequest::OdooWrite {
                    model: model("res.partner"),
                    ids: vec![7],
                    values: object(json!({
                        "name": "Acme",
                        "parent_id": 3,
                        "category_id": [[6, 0, [1, 2]]],
                        "comment": false
                    })),
                },
                ActionRequest::OdooWrite {
                    model: model("res.partner"),
                    ids: vec![8],
                    values: object(json!({
                        "name": "Beta",
                        "parent_id": false,
                        "category_id": [[6, 0, []]],
                        "comment": "note"
                    })),
                },
            ]
        );
    }

    #[test]
    fn an_odoo_write_without_a_usable_copy_is_not_revertible() {
        let write = odoo_write(json!({"name": "x"}), Some(json!([])));
        for snapshot in [
            None,
            Some(json!({"id": 7})),
            Some(json!([])),
            Some(json!([{"name": "no id"}])),
            Some(json!([{"id": 7}])),
        ] {
            assert_eq!(
                plan(&write, snapshot.as_ref(), None),
                Err(RevertRefusal::NotRevertible),
                "{snapshot:?}"
            );
        }
        let no_values = action(
            "odoo_write",
            json!({"model": "res.partner", "ids": [7]}),
            json!({"model": "res.partner", "ids": [7]}),
            Some(json!([])),
        );
        assert_eq!(
            plan(&no_values, Some(&json!([{"id": 7, "name": "a"}])), None),
            Err(RevertRefusal::NotRevertible)
        );
        let no_after = odoo_write(json!({"name": "x"}), None);
        assert_eq!(
            plan(&no_after, Some(&json!([{"id": 7, "name": "a"}])), None),
            Err(RevertRefusal::NotRevertible)
        );
    }

    #[test]
    fn an_odoo_create_is_undone_by_deleting_the_created_ids() {
        let created = odoo_create(Some(json!([{"id": 31, "name": "Neuf"}])));

        let inverse = planned(&created, None, Some("19.0"));

        assert_eq!(
            inverse.check,
            Some(ActionRequest::OdooRead {
                model: model("res.partner"),
                ids: vec![31],
                fields: vec![],
            })
        );
        assert_eq!(
            inverse.writes,
            [ActionRequest::OdooDelete {
                model: model("res.partner"),
                ids: vec![31],
            }]
        );
    }

    #[test]
    fn an_odoo_create_needs_the_ids_it_created() {
        for after in [
            None,
            Some(json!([])),
            Some(json!({"id": 31})),
            Some(json!([{"name": "no id"}])),
        ] {
            assert_eq!(
                plan(&odoo_create(after.clone()), None, None),
                Err(RevertRefusal::NotRevertible),
                "{after:?}"
            );
        }
        let many: Vec<Value> = (1..=51).map(|id| json!({"id": id})).collect();
        assert_eq!(
            plan(&odoo_create(Some(Value::Array(many))), None, None),
            Err(RevertRefusal::NotRevertible)
        );
    }

    #[test]
    fn an_n8n_update_puts_the_old_content_back_and_republishes_what_was_published() {
        let updated = workflow_action("n8n_update_workflow", Some(json!({"updatedAt": "x"})));

        for active in [true, false] {
            let inverse = planned(&updated, Some(&workflow(active)), Some("2"));

            assert_eq!(
                inverse.check,
                Some(ActionRequest::N8nGetWorkflow {
                    workflow_id: id("wf1")
                })
            );
            assert_eq!(
                inverse.writes,
                [ActionRequest::N8nUpdateWorkflow {
                    workflow_id: id("wf1"),
                    name: Some("Relance factures".into()),
                    nodes: vec![json!({"name": "Start", "type": "n8n-nodes-base.manualTrigger"})],
                    connections: object(json!({"Start": {"main": []}})),
                    settings: Some(object(json!({"executionOrder": "v1"}))),
                    publish: active,
                }]
            );
        }
    }

    #[test]
    fn an_n8n_update_without_a_usable_copy_is_not_revertible() {
        let updated = workflow_action("n8n_update_workflow", Some(json!({"updatedAt": "x"})));
        let mut without_nodes = workflow(true);
        without_nodes.as_object_mut().unwrap().remove("nodes");
        let mut without_name = workflow(true);
        without_name.as_object_mut().unwrap().remove("name");

        for snapshot in [
            None,
            Some(without_nodes),
            Some(without_name),
            Some(json!([])),
        ] {
            assert_eq!(
                plan(&updated, snapshot.as_ref(), Some("2")),
                Err(RevertRefusal::NotRevertible)
            );
        }
        assert_eq!(
            plan(
                &workflow_action("n8n_update_workflow", None),
                Some(&workflow(true)),
                Some("2")
            ),
            Err(RevertRefusal::NotRevertible)
        );
    }

    #[test]
    fn publishing_is_undone_by_unpublishing_and_the_other_way_round() {
        let published = workflow_action("n8n_publish_workflow", Some(json!({"active": true})));
        let unpublished = workflow_action("n8n_unpublish_workflow", Some(json!({"active": false})));
        let check = Some(ActionRequest::N8nGetWorkflow {
            workflow_id: id("wf1"),
        });

        let back = planned(&published, Some(&json!({"active": false})), None);
        let forth = planned(&unpublished, Some(&json!({"active": true})), None);
        let without_copy = planned(&published, None, None);

        assert_eq!(back.check, check);
        assert_eq!(
            back.writes,
            [ActionRequest::N8nUnpublishWorkflow {
                workflow_id: id("wf1")
            }]
        );
        assert_eq!(forth.check, check);
        assert_eq!(
            forth.writes,
            [ActionRequest::N8nPublishWorkflow {
                workflow_id: id("wf1")
            }]
        );
        assert_eq!(without_copy.writes, back.writes);
    }

    #[test]
    fn publishing_a_workflow_that_was_already_published_has_nothing_to_undo() {
        let published = workflow_action("n8n_publish_workflow", Some(json!({"active": true})));
        let unpublished = workflow_action("n8n_unpublish_workflow", Some(json!({"active": false})));

        assert_eq!(
            plan(&published, Some(&json!({"active": true})), None),
            Err(RevertRefusal::NotRevertible)
        );
        assert_eq!(
            plan(&unpublished, Some(&json!({"active": false})), None),
            Err(RevertRefusal::NotRevertible)
        );
        assert_eq!(
            plan(&workflow_action("n8n_publish_workflow", None), None, None),
            Err(RevertRefusal::NotRevertible)
        );
    }

    #[test]
    fn archiving_is_undone_by_unarchiving() {
        let archived = workflow_action("n8n_archive_workflow", Some(json!({"isArchived": true})));

        let inverse = planned(&archived, Some(&workflow(false)), Some("2"));

        assert_eq!(
            inverse.check,
            Some(ActionRequest::N8nGetWorkflow {
                workflow_id: id("wf1")
            })
        );
        assert_eq!(
            inverse.writes,
            [ActionRequest::N8nUnarchiveWorkflow {
                workflow_id: id("wf1")
            }]
        );
    }

    #[test]
    fn workflow_tags_are_put_back_from_the_copy_before() {
        let tagged = action(
            "n8n_set_workflow_tags",
            json!({"workflow_id": "wf1"}),
            json!({"workflow_id": "wf1", "tag_ids": ["9"]}),
            Some(json!([{"id": "9", "name": "urgent"}])),
        );
        let before = json!([{"id": "1", "name": "finance"}, {"id": "2", "name": "relance"}]);

        for snapshot in [before.clone(), json!({"tags": before})] {
            let inverse = planned(&tagged, Some(&snapshot), Some("2"));

            assert_eq!(
                inverse.check,
                Some(ActionRequest::N8nGetWorkflowTags {
                    workflow_id: id("wf1")
                })
            );
            assert_eq!(
                inverse.writes,
                [ActionRequest::N8nSetWorkflowTags {
                    workflow_id: id("wf1"),
                    tag_ids: vec![id("1"), id("2")],
                }]
            );
        }
        let none = planned(&tagged, Some(&json!([])), Some("2"));
        assert_eq!(
            none.writes,
            [ActionRequest::N8nSetWorkflowTags {
                workflow_id: id("wf1"),
                tag_ids: vec![],
            }]
        );
        assert_eq!(
            plan(&tagged, None, Some("2")),
            Err(RevertRefusal::NotRevertible)
        );
        assert_eq!(
            plan(&tagged, Some(&json!([{"name": "no id"}])), Some("2")),
            Err(RevertRefusal::NotRevertible)
        );
    }

    #[test]
    fn a_created_tag_is_undone_by_deleting_it_without_a_check() {
        let created = action(
            "n8n_create_tag",
            json!({"tag_name": "urgent"}),
            json!({"name": "urgent"}),
            Some(json!({"id": "14", "name": "urgent"})),
        );

        let inverse = planned(&created, None, Some("1"));

        assert_eq!(inverse.check, None);
        assert_eq!(
            inverse.writes,
            [ActionRequest::N8nDeleteTag { tag_id: id("14") }]
        );
        let numeric = action(
            "n8n_create_tag",
            json!({"tag_name": "urgent"}),
            json!({"name": "urgent"}),
            Some(json!({"id": 15})),
        );
        assert_eq!(
            planned(&numeric, None, None).writes,
            [ActionRequest::N8nDeleteTag { tag_id: id("15") }]
        );
        let unknown = action(
            "n8n_create_tag",
            json!({"tag_name": "urgent"}),
            json!({"name": "urgent"}),
            None,
        );
        assert_eq!(
            plan(&unknown, None, None),
            Err(RevertRefusal::NotRevertible)
        );
    }

    #[test]
    fn a_created_workflow_is_undone_by_archiving_it_on_n8n_2_only() {
        let created = action(
            "n8n_create_workflow",
            json!({"workflow_name": "Relance"}),
            json!({"name": "Relance"}),
            Some(json!({"id": "wf9", "name": "Relance"})),
        );

        let inverse = planned(&created, None, Some("2"));

        assert_eq!(
            inverse.check,
            Some(ActionRequest::N8nGetWorkflow {
                workflow_id: id("wf9")
            })
        );
        assert_eq!(
            inverse.writes,
            [ActionRequest::N8nArchiveWorkflow {
                workflow_id: id("wf9")
            }]
        );
        for version in [Some("1"), None] {
            assert_eq!(
                plan(&created, None, version),
                Err(RevertRefusal::NotRevertible)
            );
        }
    }

    #[test]
    fn only_a_succeeded_revertible_original_can_be_restored() {
        let created = odoo_create(Some(json!([{"id": 31}])));
        assert!(plan(&created, None, None).is_ok());
        assert_eq!(ensure_revertible(&created), Ok(()));

        for outcome in [Outcome::Pending, Outcome::Denied, Outcome::Failed] {
            let mut row = created.clone();
            row.outcome = outcome;
            assert_eq!(plan(&row, None, None), Err(RevertRefusal::NotRevertible));
            assert_eq!(ensure_revertible(&row), Err(RevertRefusal::NotRevertible));
        }
        let mut flagged = created.clone();
        flagged.revertible = false;
        assert_eq!(
            plan(&flagged, None, None),
            Err(RevertRefusal::NotRevertible)
        );
        let mut restore = created.clone();
        restore.reverts_action_id = Some(IntegrationActionId::new());
        assert_eq!(
            plan(&restore, None, None),
            Err(RevertRefusal::NotRevertible)
        );
    }

    #[test]
    fn tools_without_an_inverse_are_not_revertible() {
        for tool in [
            "odoo_delete",
            "odoo_call_method",
            "odoo_read",
            "n8n_get_workflow",
            "n8n_retry_execution",
            "n8n_stop_execution",
            "n8n_unarchive_workflow",
            "n8n_delete_tag",
            "unknown_tool",
        ] {
            let row = action(
                tool,
                json!({"model": "res.partner", "workflow_id": "wf1"}),
                json!({}),
                Some(json!([{"id": 1}])),
            );
            assert_eq!(
                plan(&row, Some(&json!([{"id": 1, "name": "a"}])), Some("2")),
                Err(RevertRefusal::NotRevertible),
                "{tool}"
            );
        }
    }

    #[test]
    fn an_odoo_write_is_unchanged_while_its_fields_still_read_as_after() {
        let write = odoo_write(json!({"name": "Acme SA", "parent_id": 9}), None);
        let mut write = write;
        write.after = Some(json!([
            {"id": 7, "name": "Acme SA", "parent_id": [9, "Mère"]},
            {"id": 8, "name": "Acme SA", "parent_id": [9, "Mère"]}
        ]));

        let same = json!([
            {"parent_id": [9, "Mère"], "id": 8, "name": "Acme SA", "write_date": "2026-10-02"},
            {"name": "Acme SA", "id": 7, "parent_id": [9, "Mère"], "write_date": "2026-10-03"}
        ]);
        let renamed = json!([
            {"id": 7, "name": "Acme", "parent_id": [9, "Mère"]},
            {"id": 8, "name": "Acme SA", "parent_id": [9, "Mère"]}
        ]);
        let moved = json!([
            {"id": 7, "name": "Acme SA", "parent_id": [9, "Mère"]},
            {"id": 8, "name": "Acme SA", "parent_id": [4, "Autre"]}
        ]);
        let missing = json!([{"id": 7, "name": "Acme SA", "parent_id": [9, "Mère"]}]);

        assert!(unchanged(&write, &same));
        assert!(!unchanged(&write, &renamed));
        assert!(!unchanged(&write, &moved));
        assert!(!unchanged(&write, &missing));
        assert!(!unchanged(&write, &json!([])));
        assert!(!unchanged(&write, &json!({"id": 7})));
        write.after = None;
        assert!(!unchanged(&write, &same));
    }

    #[test]
    fn an_odoo_comparison_goes_through_the_same_masking_as_the_journal() {
        let mut write = odoo_write(json!({"api_key": "k-new", "name": "Acme"}), None);
        write.after = Some(json!([
            {"id": 7, "api_key": MASK, "name": "Acme"},
            {"id": 8, "api_key": MASK, "name": "Acme"}
        ]));
        let current = json!([
            {"id": 7, "api_key": "k-new", "name": "Acme"},
            {"id": 8, "api_key": "k-new", "name": "Acme"}
        ]);

        assert!(unchanged(&write, &current));
    }

    #[test]
    fn an_odoo_create_is_unchanged_while_the_created_record_keeps_its_values() {
        let mut created = odoo_create(None);
        created.after = Some(json!([{"id": 31, "name": "Neuf", "email": "n@acme.test"}]));

        let intact = json!([{
            "id": 31, "name": "Neuf", "email": "n@acme.test", "phone": "+33", "write_date": "x"
        }]);
        let edited = json!([{"id": 31, "name": "Neuf", "email": "autre@acme.test"}]);

        assert!(unchanged(&created, &intact));
        assert!(!unchanged(&created, &edited));
        assert!(!unchanged(&created, &json!([])));
    }

    #[test]
    fn an_n8n_workflow_is_unchanged_while_its_stamps_are_those_of_after() {
        for tool in ["n8n_update_workflow", "n8n_create_workflow"] {
            let row = workflow_action(
                tool,
                Some(json!({"updatedAt": "2026-10-01T10:00:00.000Z", "versionId": "v2"})),
            );
            let same = json!({"updatedAt": "2026-10-01T10:00:00.000Z", "versionId": "v2", "name": "Autre"});
            let touched = json!({"updatedAt": "2026-10-01T11:00:00.000Z", "versionId": "v2"});
            let republished = json!({"updatedAt": "2026-10-01T10:00:00.000Z", "versionId": "v3"});
            let no_version = json!({"updatedAt": "2026-10-01T10:00:00.000Z"});
            let no_stamp = json!({"versionId": "v2"});

            assert!(unchanged(&row, &same), "{tool}");
            assert!(!unchanged(&row, &touched), "{tool}");
            assert!(!unchanged(&row, &republished), "{tool}");
            assert!(unchanged(&row, &no_version), "{tool}");
            assert!(!unchanged(&row, &no_stamp), "{tool}");
        }
        let unstamped = workflow_action("n8n_update_workflow", Some(json!({"versionId": "v2"})));
        assert!(!unchanged(&unstamped, &json!({"versionId": "v2"})));
        assert!(!unchanged(
            &workflow_action("n8n_update_workflow", None),
            &json!({"updatedAt": "x"})
        ));
    }

    #[test]
    fn a_publication_is_unchanged_while_the_workflow_is_still_in_the_state_it_left() {
        let published = workflow_action("n8n_publish_workflow", Some(json!({"active": true})));
        let unpublished = workflow_action("n8n_unpublish_workflow", Some(json!({"active": false})));

        assert!(unchanged(&published, &json!({"active": true, "name": "x"})));
        assert!(!unchanged(&published, &json!({"active": false})));
        assert!(unchanged(&unpublished, &json!({"active": false})));
        assert!(!unchanged(&unpublished, &json!({"active": true})));
        assert!(unchanged(&published, &json!({"activeVersionId": "v1"})));
        assert!(!unchanged(&published, &json!({"activeVersionId": null})));
        assert!(!unchanged(&published, &json!({"name": "no state"})));
    }

    #[test]
    fn an_archived_workflow_is_unchanged_while_it_stays_archived() {
        let archived = workflow_action("n8n_archive_workflow", Some(json!({"isArchived": true})));

        assert!(unchanged(&archived, &json!({"isArchived": true})));
        assert!(!unchanged(&archived, &json!({"isArchived": false})));
        assert!(!unchanged(&archived, &json!({"name": "x"})));
    }

    #[test]
    fn workflow_tags_are_unchanged_while_they_are_the_same_set() {
        let tagged = action(
            "n8n_set_workflow_tags",
            json!({"workflow_id": "wf1"}),
            json!({"workflow_id": "wf1", "tag_ids": ["1", "2"]}),
            Some(json!([{"id": "1", "name": "a"}, {"id": "2", "name": "b"}])),
        );

        assert!(unchanged(
            &tagged,
            &json!([{"id": "2", "name": "renamed"}, {"id": "1", "name": "a"}])
        ));
        assert!(!unchanged(&tagged, &json!([{"id": "1", "name": "a"}])));
        assert!(!unchanged(
            &tagged,
            &json!([{"id": "1"}, {"id": "2"}, {"id": "3"}])
        ));
        assert!(!unchanged(&tagged, &json!({"id": "1"})));
    }

    #[test]
    fn a_created_tag_has_nothing_to_compare() {
        let created = action(
            "n8n_create_tag",
            json!({"tag_name": "urgent"}),
            json!({"name": "urgent"}),
            Some(json!({"id": "14"})),
        );

        assert!(unchanged(&created, &Value::Null));
    }

    #[test]
    fn a_tool_without_an_inverse_is_never_unchanged() {
        let read = action("odoo_read", json!({}), json!({}), Some(json!([{"id": 1}])));

        assert!(!unchanged(&read, &json!([{"id": 1}])));
    }

    #[test]
    fn a_refusal_says_which_status_family_it_belongs_to() {
        let conflicts = [
            RevertRefusal::AlreadyReverted,
            RevertRefusal::InterventionRunning,
            RevertRefusal::ChangedSince,
        ];
        let invalids = [
            RevertRefusal::NotRevertible,
            RevertRefusal::InstanceArchived,
            RevertRefusal::ProductionWriteNotConfirmed,
        ];

        assert!(conflicts.iter().all(|refusal| refusal.is_conflict()));
        assert!(invalids.iter().all(|refusal| !refusal.is_conflict()));
        assert_eq!(RevertRefusal::ChangedSince.code(), "changed_since");
    }
}
