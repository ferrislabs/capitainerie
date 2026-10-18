DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'capitainerie_egress_proxy') THEN
        CREATE ROLE capitainerie_egress_proxy LOGIN;
    END IF;
END
$$;

GRANT USAGE ON SCHEMA public TO capitainerie_egress_proxy;

REVOKE ALL ON ALL TABLES IN SCHEMA public FROM capitainerie_egress_proxy;

GRANT SELECT ON task_grants, provider_credentials, github_installations
    TO capitainerie_egress_proxy;
