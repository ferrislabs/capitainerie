@id("human-calls-granted-connectors")
permit (
  principal,
  action == Capitainerie::Action::"mcp.call",
  resource
)
when {
  context.actor_kind == "human" && !(context has actor) &&
  context.organization == resource.organization &&
  context.initiated &&
  context.granted
};
