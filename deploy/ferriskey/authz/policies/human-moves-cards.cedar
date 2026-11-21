@id("human-moves-cards")
permit (
  principal,
  action == Capitainerie::Action::"card.move",
  resource
)
when {
  context.actor_kind == "human" && !(context has actor) &&
  context.organization == resource.organization &&
  ["proposed", "todo", "in_progress", "done", "rejected"].contains(context.from) &&
  ["proposed", "todo", "in_progress", "done", "rejected"].contains(context.to)
};
