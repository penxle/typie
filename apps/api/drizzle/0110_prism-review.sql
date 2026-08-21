CREATE TYPE "public"."_prism_reaction" AS ENUM('UP', 'DOWN');
CREATE TYPE "public"."_prism_review_tier" AS ENUM('LOW', 'MEDIUM', 'HIGH');
CREATE TYPE "public"."_prism_workflow_state" AS ENUM('RUNNING', 'COMPLETED', 'FAILED', 'CANCELED');
CREATE TABLE "prism_review_document_versions" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"version" integer NOT NULL,
	"title" text,
	"subtitle" text,
	"content" text NOT NULL,
	"character_count" integer NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_review_document_versions_document_id_version_unique" UNIQUE("document_id","version")
);

CREATE TABLE "prism_review_rounds" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"round" integer NOT NULL,
	"session_id" text,
	"prism_run_seq" integer NOT NULL,
	"workflow_id" text,
	"closed_at" timestamp with time zone,
	"tier" "_prism_review_tier" NOT NULL,
	"document_version_id" text NOT NULL,
	"result" jsonb,
	"reaction" "_prism_reaction",
	"reaction_note" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_review_rounds_workflow_id_unique" UNIQUE("workflow_id"),
	CONSTRAINT "prism_review_rounds_document_id_round_unique" UNIQUE("document_id","round")
);

CREATE TABLE "prism_workflows" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"prism_workflow_id" text NOT NULL,
	"app" text NOT NULL,
	"name" text NOT NULL,
	"ref" text,
	"state" "_prism_workflow_state" DEFAULT 'RUNNING' NOT NULL,
	"started_at" timestamp with time zone DEFAULT now() NOT NULL,
	"finished_at" timestamp with time zone,
	"usage" jsonb,
	"error" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_workflows_prism_workflow_id_unique" UNIQUE("prism_workflow_id")
);

ALTER TABLE "prism_review_document_versions" ADD CONSTRAINT "prism_review_document_versions_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_rounds" ADD CONSTRAINT "prism_review_rounds_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_rounds" ADD CONSTRAINT "prism_review_rounds_session_id_prism_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."prism_sessions"("id") ON DELETE set null ON UPDATE cascade;
ALTER TABLE "prism_review_rounds" ADD CONSTRAINT "prism_review_rounds_workflow_id_prism_workflows_id_fk" FOREIGN KEY ("workflow_id") REFERENCES "public"."prism_workflows"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_rounds" ADD CONSTRAINT "prism_review_rounds_document_version_id_prism_review_document_versions_id_fk" FOREIGN KEY ("document_version_id") REFERENCES "public"."prism_review_document_versions"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_workflows" ADD CONSTRAINT "prism_workflows_session_id_prism_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."prism_sessions"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "prism_review_rounds_session_id_index" ON "prism_review_rounds" USING btree ("session_id");
CREATE INDEX "prism_workflows_session_id_index" ON "prism_workflows" USING btree ("session_id");
CREATE INDEX "prism_workflows_state_index" ON "prism_workflows" USING btree ("state");