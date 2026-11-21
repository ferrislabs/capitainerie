@id("integrator-calls-reads-and-comments")
permit (
  principal,
  action in [
    Capitainerie::Action::"integration.call",
    Capitainerie::Action::"card.read",
    Capitainerie::Action::"card.comment"
  ],
  resource
)
when {
  context.actor_kind == "integrator" && context has actor &&
  context.organization == resource.organization
};
