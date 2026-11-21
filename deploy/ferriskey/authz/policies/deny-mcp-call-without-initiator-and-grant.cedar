@id("deny-mcp-call-without-initiator-and-grant")
forbid (
  principal,
  action == Capitainerie::Action::"mcp.call",
  resource
)
unless {
  context.initiated && context.granted
};
