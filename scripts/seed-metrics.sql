-- Données de démonstration pour la page Métriques de la console.
--
--   psql "$DATABASE_URL" -v org=capitainerie -f scripts/seed-metrics.sql
--   psql "$DATABASE_URL" -v org=capitainerie -v clean=1 -f scripts/seed-metrics.sql   # retire tout
--
-- Tout ce qui est créé porte `created_by = 'seed-metrics'` (agents, projets, cartes, transcripts)
-- ou un `initiated_by` listé plus bas ; relancer le script remplace le jeu précédent.
-- À ne jamais lancer sur une base de production.

\if :{?org}
\else
  \set org capitainerie
\endif
\if :{?clean}
\else
  \set clean 0
\endif

BEGIN;

CREATE TEMP TABLE seed_org AS SELECT id FROM organizations WHERE slug = :'org';
DO $$ BEGIN
  IF (SELECT count(*) FROM seed_org) <> 1 THEN
    RAISE EXCEPTION 'organisation introuvable : voir -v org=<slug>';
  END IF;
END $$;

CREATE TEMP TABLE seed_users (name text);
INSERT INTO seed_users VALUES ('baptiste.parmantier'), ('alice.martin'), ('karim.benali'), ('lea.dubois');

-- Retire le jeu précédent, dans l'ordre des clés étrangères.
DELETE FROM run_steps WHERE run_id IN (SELECT id FROM runs WHERE initiated_by IN (SELECT name FROM seed_users));
DELETE FROM agent_tasks WHERE created_by IN (SELECT name FROM seed_users);
DELETE FROM runs WHERE initiated_by IN (SELECT name FROM seed_users);
DELETE FROM transcripts WHERE submitted_by = 'seed-metrics';
DELETE FROM cards WHERE created_by_subject = 'seed-metrics';
DELETE FROM projects WHERE created_by = 'seed-metrics';
DELETE FROM agents WHERE created_by = 'seed-metrics';

\if :clean
  COMMIT;
  \quit
\endif

SELECT setseed(0.42);

INSERT INTO agents (id, organization_id, slug, name, role, model, hue, created_by, created_at, updated_at)
SELECT gen_random_uuid(), o.id, v.slug, v.name, v.role, v.model, v.hue, 'seed-metrics', now(), now()
FROM seed_org o,
     (VALUES ('iris-relais-retours', 'Iris (Relais retours)', 'Support', 'claude-sonnet-5-5', 200),
             ('marcel-support', 'Marcel (Support client)', 'Support', 'claude-haiku-5-5', 40),
             ('nova-veille', 'Nova (Veille légale)', 'Veille', 'claude-opus-5-5', 290)) AS v(slug, name, role, model, hue);

INSERT INTO projects (id, organization_id, name, created_by, created_at, updated_at)
SELECT gen_random_uuid(), o.id, v.name, 'seed-metrics', now(), now()
FROM seed_org o, (VALUES ('Refonte du site'), ('Support Q4'), ('Veille réglementaire')) AS v(name);

-- Douze cartes réparties sur les projets, dont trois hors projet.
INSERT INTO cards (id, organization_id, title, description, "column", created_by_subject, version, created_at, updated_at, project_id)
SELECT gen_random_uuid(), o.id, t.title, '', 'in_progress', 'seed-metrics', 1, now(), now(),
       (SELECT p.id FROM projects p WHERE p.organization_id = o.id AND p.name = t.project)
FROM seed_org o,
     (VALUES ('Relancer les factures impayées', 'Support Q4'),
             ('Répondre aux demandes de retour', 'Support Q4'),
             ('Trier le backlog support', 'Support Q4'),
             ('Maquetter la page d''accueil', 'Refonte du site'),
             ('Migrer le blog', 'Refonte du site'),
             ('Rédiger les mentions légales', 'Refonte du site'),
             ('Suivre le décret du 14 octobre', 'Veille réglementaire'),
             ('Surveiller les sous-domaines', 'Veille réglementaire'),
             ('Résumer la jurisprudence RGPD', 'Veille réglementaire'),
             ('Préparer le comité de pilotage', NULL),
             ('Mettre à jour le registre', NULL),
             ('Nettoyer les doublons clients', NULL)) AS t(title, project);

-- Tarif en micro-dollars par jeton (entrée, sortie).
CREATE TEMP TABLE seed_models (model text, price_in int, price_out int);
INSERT INTO seed_models VALUES ('claude-haiku-5-5', 1, 5), ('claude-sonnet-5-5', 3, 15), ('claude-opus-5-5', 5, 25);

-- 70 runs d'agent sur les 28 derniers jours : une carte, un agent, un utilisateur, un modèle.
CREATE TEMP TABLE seed_agent_runs AS
SELECT gen_random_uuid() AS run_id,
       c.id AS card_id,
       a.id AS agent_id,
       a.slug AS agent_slug,
       (SELECT name FROM seed_users ORDER BY random() LIMIT 1) AS username,
       m.model, m.price_in, m.price_out,
       now() - (random() * interval '28 days') AS started,
       (2000 + random() * 38000)::bigint AS tokens_in,
       (300 + random() * 6000)::bigint AS tokens_out
FROM generate_series(1, 70) AS n
CROSS JOIN LATERAL (SELECT id FROM cards WHERE created_by_subject = 'seed-metrics' ORDER BY random() + n * 0 LIMIT 1) AS c
CROSS JOIN LATERAL (SELECT id, slug FROM agents WHERE created_by = 'seed-metrics' ORDER BY random() + n * 0 LIMIT 1) AS a
CROSS JOIN LATERAL (SELECT * FROM seed_models ORDER BY random() + n * 0 LIMIT 1) AS m;

INSERT INTO runs (id, organization_id, initiated_by, status, pipeline, created_at, started_at, finished_at, card_id)
SELECT r.run_id, o.id, r.username, 'succeeded', '{}', r.started, r.started, r.started + interval '40 seconds', r.card_id
FROM seed_agent_runs r, seed_org o;

INSERT INTO agent_tasks (run_id, organization_id, card_id, agent_id, allow_writes, created_by, created_at)
SELECT r.run_id, o.id, r.card_id, r.agent_id, false, r.username, r.started
FROM seed_agent_runs r, seed_org o;

INSERT INTO run_steps (id, run_id, organization_id, seq, kind, agent, status, attempt, model,
                       input_tokens, output_tokens, cost_micros, started_at, finished_at)
SELECT gen_random_uuid(), r.run_id, o.id, 0, 'agent_task', r.agent_slug, 'succeeded', 1, r.model,
       r.tokens_in, r.tokens_out, r.tokens_in * r.price_in + r.tokens_out * r.price_out,
       r.started, r.started + interval '40 seconds'
FROM seed_agent_runs r, seed_org o;

-- 18 transcripts traités par le pipeline (trois étapes), un tiers sans projet.
CREATE TEMP TABLE seed_pipeline_runs AS
SELECT gen_random_uuid() AS run_id,
       gen_random_uuid() AS transcript_id,
       CASE WHEN n % 3 = 0 THEN NULL
            ELSE (SELECT id FROM projects WHERE created_by = 'seed-metrics' ORDER BY name OFFSET (n % 3) - 1 LIMIT 1) END AS project_id,
       (SELECT name FROM seed_users ORDER BY random() + n * 0 LIMIT 1) AS username,
       now() - (random() * interval '28 days') AS started
FROM generate_series(1, 18) AS n;

INSERT INTO transcripts (id, organization_id, submitted_by, title, body, created_at, project_id)
SELECT p.transcript_id, o.id, 'seed-metrics', 'Réunion ' || left(p.transcript_id::text, 4), 'Compte rendu de démonstration.', p.started, p.project_id
FROM seed_pipeline_runs p, seed_org o;

INSERT INTO runs (id, organization_id, transcript_id, initiated_by, status, pipeline, created_at, started_at, finished_at)
SELECT p.run_id, o.id, p.transcript_id, p.username, 'succeeded', ARRAY['extractor', 'deduplicator', 'writer'], p.started, p.started, p.started + interval '90 seconds'
FROM seed_pipeline_runs p, seed_org o;

INSERT INTO run_steps (id, run_id, organization_id, seq, kind, agent, status, attempt, model,
                       input_tokens, output_tokens, cost_micros, started_at, finished_at)
SELECT gen_random_uuid(), p.run_id, o.id, s.seq, s.kind, s.agent, 'succeeded', 1, s.model,
       s.tin, s.tout, s.tin * m.price_in + s.tout * m.price_out,
       p.started + (s.seq * interval '25 seconds'), p.started + ((s.seq + 1) * interval '25 seconds')
FROM seed_pipeline_runs p, seed_org o,
     (VALUES (0, 'extract', 'extractor', 'claude-sonnet-5-5', 18000, 2500),
             (1, 'deduplicate', 'deduplicator', 'claude-haiku-5-5', 6000, 800),
             (2, 'write', 'writer', 'claude-sonnet-5-5', 9000, 3000)) AS s(seq, kind, agent, model, tin, tout)
JOIN seed_models m ON m.model = s.model;

COMMIT;

SELECT 'agents' AS what, count(*) FROM agents WHERE created_by = 'seed-metrics'
UNION ALL SELECT 'projets', count(*) FROM projects WHERE created_by = 'seed-metrics'
UNION ALL SELECT 'cartes', count(*) FROM cards WHERE created_by_subject = 'seed-metrics'
UNION ALL SELECT 'runs', count(*) FROM runs WHERE initiated_by IN (SELECT name FROM seed_users)
UNION ALL SELECT 'coût ($)', round(sum(cost_micros) / 1e6, 2)::bigint
  FROM run_steps WHERE run_id IN (SELECT id FROM runs WHERE initiated_by IN (SELECT name FROM seed_users));
