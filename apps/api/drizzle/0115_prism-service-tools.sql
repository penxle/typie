CREATE TYPE "public"."_prism_tool_policy" AS ENUM('READ_ONLY', 'STANDARD', 'FULL');
CREATE TABLE "prism_tool_calls" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"tool_call_id" text NOT NULL,
	"tool" text NOT NULL,
	"result" jsonb,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "prism_runs" ADD COLUMN "site_id" text;
ALTER TABLE "prism_sessions" ADD COLUMN "tool_policy" "_prism_tool_policy" DEFAULT 'STANDARD' NOT NULL;
ALTER TABLE "prism_tool_calls" ADD CONSTRAINT "prism_tool_calls_session_id_prism_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."prism_sessions"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "prism_tool_calls_session_id_tool_call_id_index" ON "prism_tool_calls" USING btree ("session_id","tool_call_id");
CREATE INDEX "prism_tool_calls_session_id_index" ON "prism_tool_calls" USING btree ("session_id");
ALTER TABLE "prism_runs" ADD CONSTRAINT "prism_runs_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;