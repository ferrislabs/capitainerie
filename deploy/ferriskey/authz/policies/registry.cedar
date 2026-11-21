@id("registry-agent-reads-and-comments")
permit (
  principal,
  action in [Capitainerie::Action::"card.read", Capitainerie::Action::"card.comment"],
  resource
)
when { context.actor_kind == "registry" };

@id("registry-agent-calls-connectors")
permit (principal, action == Capitainerie::Action::"mcp.call", resource)
when { context.actor_kind == "registry" };
