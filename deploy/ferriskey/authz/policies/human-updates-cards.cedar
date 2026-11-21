@id("human-updates-cards")
permit (
  principal,
  action == Capitainerie::Action::"card.update",
  resource
)
when {
  context.actor_kind == "human" && !(context has actor) &&
  context.organization == resource.organization &&
  (
    (
      context.intent == "update" &&
      context has column &&
      ["proposed", "todo", "in_progress", "done", "rejected"].contains(context.column)
    ) ||
    (["link", "edit"].contains(context.intent) && !(context has column))
  )
};
