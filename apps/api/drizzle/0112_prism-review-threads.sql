CREATE TYPE "public"."_prism_review_comment_author" AS ENUM('USER', 'AI');
CREATE TYPE "public"."_prism_review_pass" AS ENUM('JUDGMENT', 'STYLISTIC');
CREATE TYPE "public"."_prism_review_thread_state" AS ENUM('OPEN', 'CLOSED', 'RESOLVED', 'WITHDRAWN');
CREATE TABLE "prism_review_thread_comments" (
	"id" text PRIMARY KEY NOT NULL,
	"thread_id" text NOT NULL,
	"author" "_prism_review_comment_author" NOT NULL,
	"user_id" text NOT NULL,
	"body" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "prism_review_threads" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"born_round" integer NOT NULL,
	"round_id" text NOT NULL,
	"issue_index" integer NOT NULL,
	"issue_id" text,
	"trait" text NOT NULL,
	"pass" "_prism_review_pass" NOT NULL,
	"body" text,
	"anchors" jsonb NOT NULL,
	"state" "_prism_review_thread_state" DEFAULT 'OPEN' NOT NULL,
	"state_changed_at" timestamp with time zone,
	"reaction" "_prism_reaction",
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_review_threads_document_id_born_round_issue_index_unique" UNIQUE("document_id","born_round","issue_index")
);

ALTER TABLE "prism_review_thread_comments" ADD CONSTRAINT "prism_review_thread_comments_thread_id_prism_review_threads_id_fk" FOREIGN KEY ("thread_id") REFERENCES "public"."prism_review_threads"("id") ON DELETE cascade ON UPDATE cascade;
ALTER TABLE "prism_review_thread_comments" ADD CONSTRAINT "prism_review_thread_comments_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_threads" ADD CONSTRAINT "prism_review_threads_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_threads" ADD CONSTRAINT "prism_review_threads_round_id_prism_review_rounds_id_fk" FOREIGN KEY ("round_id") REFERENCES "public"."prism_review_rounds"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "prism_review_thread_comments_thread_id_created_at_index" ON "prism_review_thread_comments" USING btree ("thread_id","created_at");
CREATE INDEX "prism_review_threads_round_id_index" ON "prism_review_threads" USING btree ("round_id");