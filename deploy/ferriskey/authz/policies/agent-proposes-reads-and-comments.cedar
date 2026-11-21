@id("agent-proposes-reads-and-comments")
permit (
  principal,
  action in [
    Capitainerie::Action::"card.create",
    Capitainerie::Action::"card.read",
    Capitainerie::Action::"card.comment"
  ],
  resource
)
when {
  context.actor_kind == "agent" && context has actor &&
  context.organization == resource.organization
};
