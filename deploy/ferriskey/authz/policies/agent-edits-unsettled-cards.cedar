@id("agent-edits-unsettled-cards")
permit (
  principal,
  action == Capitainerie::Action::"card.update",
  resource
)
when {
  context.actor_kind == "agent" && context has actor &&
  context.organization == resource.organization &&
  context.intent == "update" &&
  context has column &&
  ["proposed", "todo", "in_progress"].contains(context.column)
};
