CREATE TYPE "public"."_document_conflict_kind" AS ENUM('ATTRIBUTE', 'TEXT', 'LIFECYCLE', 'POSITION', 'ORDER');
CREATE TABLE "document_commits" (
	"id" text PRIMARY KEY NOT NULL,
	"commit_id" text NOT NULL,
	"document_id" text NOT NULL,
	"parent_id" text,
	"second_parent_id" text,
	"steps" jsonb NOT NULL,
	"meta" jsonb,
	"device_id" text NOT NULL,
	"user_id" text,
	"committed_at" timestamp with time zone NOT NULL,
	"pushed_at" timestamp with time zone DEFAULT now() NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "document_conflict_branches" (
	"id" text PRIMARY KEY NOT NULL,
	"conflict_id" text NOT NULL,
	"commit_id" text NOT NULL,
	"value" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "document_conflict_resolutions" (
	"id" text PRIMARY KEY NOT NULL,
	"conflict_id" text NOT NULL,
	"value" jsonb NOT NULL,
	"commit_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "document_conflict_resolutions_conflict_id_unique" UNIQUE("conflict_id")
);

CREATE TABLE "document_conflicts" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"merge_commit_id" text NOT NULL,
	"kind" "_document_conflict_kind" NOT NULL,
	"target" jsonb NOT NULL,
	"base_value" jsonb,
	"auto_resolved_branch_id" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "document_head_contents" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"json" jsonb NOT NULL,
	"text" text NOT NULL,
	"character_count" integer DEFAULT 0 NOT NULL,
	"blob_size" bigint DEFAULT 0 NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "document_head_contents_document_id_unique" UNIQUE("document_id")
);

ALTER TABLE "documents" ADD COLUMN "head_commit_id" text;
ALTER TABLE "document_commits" ADD CONSTRAINT "document_commits_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_commits" ADD CONSTRAINT "document_commits_parent_id_document_commits_id_fk" FOREIGN KEY ("parent_id") REFERENCES "public"."document_commits"("id") ON DELETE no action ON UPDATE no action;
ALTER TABLE "document_commits" ADD CONSTRAINT "document_commits_second_parent_id_document_commits_id_fk" FOREIGN KEY ("second_parent_id") REFERENCES "public"."document_commits"("id") ON DELETE no action ON UPDATE no action;
ALTER TABLE "document_commits" ADD CONSTRAINT "document_commits_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_conflict_branches" ADD CONSTRAINT "document_conflict_branches_conflict_id_document_conflicts_id_fk" FOREIGN KEY ("conflict_id") REFERENCES "public"."document_conflicts"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_conflict_branches" ADD CONSTRAINT "document_conflict_branches_commit_id_document_commits_id_fk" FOREIGN KEY ("commit_id") REFERENCES "public"."document_commits"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_conflict_resolutions" ADD CONSTRAINT "document_conflict_resolutions_conflict_id_document_conflicts_id_fk" FOREIGN KEY ("conflict_id") REFERENCES "public"."document_conflicts"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_conflict_resolutions" ADD CONSTRAINT "document_conflict_resolutions_commit_id_document_commits_id_fk" FOREIGN KEY ("commit_id") REFERENCES "public"."document_commits"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_conflicts" ADD CONSTRAINT "document_conflicts_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_conflicts" ADD CONSTRAINT "document_conflicts_merge_commit_id_document_commits_id_fk" FOREIGN KEY ("merge_commit_id") REFERENCES "public"."document_commits"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_conflicts" ADD CONSTRAINT "document_conflicts_auto_resolved_branch_id_document_conflict_branches_id_fk" FOREIGN KEY ("auto_resolved_branch_id") REFERENCES "public"."document_conflict_branches"("id") ON DELETE no action ON UPDATE no action;
ALTER TABLE "document_head_contents" ADD CONSTRAINT "document_head_contents_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "document_commits_document_id_pushed_at_index" ON "document_commits" USING btree ("document_id","pushed_at");
CREATE UNIQUE INDEX "document_commits_document_id_commit_id_index" ON "document_commits" USING btree ("document_id","commit_id");
CREATE INDEX "document_conflict_branches_conflict_id_index" ON "document_conflict_branches" USING btree ("conflict_id");
CREATE INDEX "document_conflict_branches_commit_id_index" ON "document_conflict_branches" USING btree ("commit_id");
CREATE INDEX "document_conflicts_document_id_index" ON "document_conflicts" USING btree ("document_id");
ALTER TABLE "documents" ADD CONSTRAINT "documents_head_commit_id_document_commits_id_fk" FOREIGN KEY ("head_commit_id") REFERENCES "public"."document_commits"("id") ON DELETE set null ON UPDATE cascade;