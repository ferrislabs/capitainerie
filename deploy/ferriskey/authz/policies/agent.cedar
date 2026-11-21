@id("agent-proposes-reads-and-comments")
permit (
  principal,
  action in [Capitainerie::Action::"card.create", Capitainerie::Action::"card.read", Capitainerie::Action::"card.comment"],
  resource
)
when { context.actor_kind == "agent" };

@id("agent-edits-unsettled-cards")
permit (principal, action == Capitainerie::Action::"card.update", resource)
when {
  context.actor_kind == "agent" &&
  context.intent == "update" &&
  context has column &&
  !["done", "rejected"].contains(context.column)
};

@id("agent-moves-cards")
permit (principal, action == Capitainerie::Action::"card.move", resource)
when { context.actor_kind == "agent" };
