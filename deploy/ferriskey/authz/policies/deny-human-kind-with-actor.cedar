@id("deny-human-kind-with-actor")
forbid (
  principal,
  action in Capitainerie::Action::"capitainerie.any",
  resource
)
when {
  context has actor && context.actor_kind == "human"
};
