CREATE TYPE "public"."_document_comment_state" AS ENUM('ACTIVE', 'DELETED');
CREATE TYPE "public"."_document_comment_thread_state" AS ENUM('ACTIVE', 'DELETED');
CREATE TABLE "document_comment_threads" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"user_id" text NOT NULL,
	"selection" jsonb NOT NULL,
	"state" "_document_comment_thread_state" DEFAULT 'ACTIVE' NOT NULL,
	"resolved_by" text,
	"resolved_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "document_comments" (
	"id" text PRIMARY KEY NOT NULL,
	"thread_id" text NOT NULL,
	"user_id" text NOT NULL,
	"content" text NOT NULL,
	"state" "_document_comment_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_comment_threads" ADD CONSTRAINT "document_comment_threads_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_comment_threads" ADD CONSTRAINT "document_comment_threads_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_comment_threads" ADD CONSTRAINT "document_comment_threads_resolved_by_users_id_fk" FOREIGN KEY ("resolved_by") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_comments" ADD CONSTRAINT "document_comments_thread_id_document_comment_threads_id_fk" FOREIGN KEY ("thread_id") REFERENCES "public"."document_comment_threads"("id") ON DELETE cascade ON UPDATE cascade;
ALTER TABLE "document_comments" ADD CONSTRAINT "document_comments_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "document_comment_threads_document_id_state_index" ON "document_comment_threads" USING btree ("document_id","state");
CREATE INDEX "document_comments_thread_id_created_at_index" ON "document_comments" USING btree ("thread_id","created_at");