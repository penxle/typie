CREATE TABLE "document_states" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"graph" "bytea" NOT NULL,
	"json" jsonb NOT NULL,
	"text" text NOT NULL,
	"character_count" integer DEFAULT 0 NOT NULL,
	"blob_size" bigint DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "document_states_document_id_unique" UNIQUE("document_id")
);

ALTER TABLE "document_commits" DISABLE ROW LEVEL SECURITY;
ALTER TABLE "document_conflict_branches" DISABLE ROW LEVEL SECURITY;
ALTER TABLE "document_conflict_resolutions" DISABLE ROW LEVEL SECURITY;
ALTER TABLE "document_conflicts" DISABLE ROW LEVEL SECURITY;
ALTER TABLE "document_head_contents" DISABLE ROW LEVEL SECURITY;
ALTER TABLE "document_objects" DISABLE ROW LEVEL SECURITY;
DROP TABLE "document_commits" CASCADE;
DROP TABLE "document_conflict_branches" CASCADE;
DROP TABLE "document_conflict_resolutions" CASCADE;
DROP TABLE "document_conflicts" CASCADE;
DROP TABLE "document_head_contents" CASCADE;
DROP TABLE "document_objects" CASCADE;
ALTER TABLE "documents" DROP CONSTRAINT "documents_head_commit_id_document_commits_id_fk";

ALTER TABLE "document_states" ADD CONSTRAINT "document_states_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "documents" DROP COLUMN "head_commit_id";