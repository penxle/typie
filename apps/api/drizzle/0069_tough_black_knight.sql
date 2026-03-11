CREATE TYPE "public"."_issue_priority" AS ENUM('NONE', 'LOW', 'MEDIUM', 'HIGH', 'URGENT');
CREATE TYPE "public"."_issue_state" AS ENUM('ACTIVE', 'DELETED');
CREATE TYPE "public"."_issue_status" AS ENUM('OPEN', 'IN_PROGRESS', 'RESOLVED', 'CLOSED');
CREATE TABLE "issue_entities" (
	"id" text PRIMARY KEY NOT NULL,
	"issue_id" text NOT NULL,
	"entity_id" text NOT NULL,
	CONSTRAINT "issue_entities_issue_id_entity_id_unique" UNIQUE("issue_id","entity_id")
);

CREATE TABLE "issues" (
	"id" text PRIMARY KEY NOT NULL,
	"site_id" text NOT NULL,
	"content" text NOT NULL,
	"status" "_issue_status" DEFAULT 'OPEN' NOT NULL,
	"priority" "_issue_priority" DEFAULT 'NONE' NOT NULL,
	"due_at" timestamp with time zone,
	"state" "_issue_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "issue_entities" ADD CONSTRAINT "issue_entities_issue_id_issues_id_fk" FOREIGN KEY ("issue_id") REFERENCES "public"."issues"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "issue_entities" ADD CONSTRAINT "issue_entities_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "issues" ADD CONSTRAINT "issues_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "issue_entities_entity_id_index" ON "issue_entities" USING btree ("entity_id");
CREATE INDEX "issues_site_id_state_index" ON "issues" USING btree ("site_id","state");
CREATE INDEX "issues_site_id_state_status_index" ON "issues" USING btree ("site_id","state","status");