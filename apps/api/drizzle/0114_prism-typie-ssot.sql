CREATE TYPE "public"."_prism_run_state" AS ENUM('RUNNING', 'COMPLETED', 'FAILED', 'CANCELED');
CREATE TABLE "prism_runs" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"run_seq" integer NOT NULL,
	"state" "_prism_run_state" DEFAULT 'RUNNING' NOT NULL,
	"started_at" timestamp with time zone NOT NULL,
	"finished_at" timestamp with time zone,
	"reaction" "_prism_reaction",
	"reaction_note" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_runs_session_id_run_seq_unique" UNIQUE("session_id","run_seq")
);

CREATE TABLE "prism_session_events" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"seq" integer NOT NULL,
	"kind" text NOT NULL,
	"occurred_at" timestamp with time zone NOT NULL,
	"logged_at" timestamp with time zone NOT NULL,
	"context" jsonb,
	"data" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_session_events_session_id_seq_unique" UNIQUE("session_id","seq")
);

CREATE TABLE "prism_workflow_events" (
	"id" text PRIMARY KEY NOT NULL,
	"workflow_id" text NOT NULL,
	"seq" integer NOT NULL,
	"kind" text NOT NULL,
	"occurred_at" timestamp with time zone NOT NULL,
	"logged_at" timestamp with time zone NOT NULL,
	"context" jsonb,
	"data" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_workflow_events_workflow_id_seq_unique" UNIQUE("workflow_id","seq")
);

ALTER TABLE "prism_sessions" ADD COLUMN "cursor" integer DEFAULT 0 NOT NULL;
ALTER TABLE "prism_workflows" ADD COLUMN "cursor" integer DEFAULT 0 NOT NULL;
ALTER TABLE "prism_runs" ADD CONSTRAINT "prism_runs_session_id_prism_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."prism_sessions"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_session_events" ADD CONSTRAINT "prism_session_events_session_id_prism_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."prism_sessions"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_workflow_events" ADD CONSTRAINT "prism_workflow_events_workflow_id_prism_workflows_id_fk" FOREIGN KEY ("workflow_id") REFERENCES "public"."prism_workflows"("id") ON DELETE restrict ON UPDATE cascade;