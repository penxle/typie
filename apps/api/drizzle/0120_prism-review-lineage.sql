CREATE TABLE "prism_review_lineages" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"tier" "_prism_review_tier" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "prism_review_thread_seats" (
	"id" text PRIMARY KEY NOT NULL,
	"thread_id" text NOT NULL,
	"round_id" text NOT NULL,
	"issue_index" integer NOT NULL,
	"anchors" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_review_thread_seats_thread_id_round_id_unique" UNIQUE("thread_id","round_id"),
	CONSTRAINT "prism_review_thread_seats_round_id_issue_index_unique" UNIQUE("round_id","issue_index")
);

ALTER TABLE "prism_review_threads" DROP CONSTRAINT "prism_review_threads_document_id_born_round_issue_index_unique";
ALTER TABLE "prism_review_threads" DROP CONSTRAINT "prism_review_threads_round_id_prism_review_rounds_id_fk";

DROP INDEX "prism_review_threads_round_id_index";
ALTER TABLE "prism_review_rounds" ADD COLUMN "lineage_id" text NOT NULL;
ALTER TABLE "prism_review_rounds" ADD COLUMN "base_round_id" text;
ALTER TABLE "prism_review_threads" ADD COLUMN "lineage_id" text NOT NULL;
ALTER TABLE "prism_review_threads" ADD COLUMN "born_round_id" text NOT NULL;
ALTER TABLE "prism_review_threads" ADD COLUMN "settled_round_id" text;
ALTER TABLE "prism_review_lineages" ADD CONSTRAINT "prism_review_lineages_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_thread_seats" ADD CONSTRAINT "prism_review_thread_seats_thread_id_prism_review_threads_id_fk" FOREIGN KEY ("thread_id") REFERENCES "public"."prism_review_threads"("id") ON DELETE cascade ON UPDATE cascade;
ALTER TABLE "prism_review_thread_seats" ADD CONSTRAINT "prism_review_thread_seats_round_id_prism_review_rounds_id_fk" FOREIGN KEY ("round_id") REFERENCES "public"."prism_review_rounds"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "prism_review_lineages_document_id_index" ON "prism_review_lineages" USING btree ("document_id");
CREATE INDEX "prism_review_thread_seats_round_id_index" ON "prism_review_thread_seats" USING btree ("round_id");
ALTER TABLE "prism_review_rounds" ADD CONSTRAINT "prism_review_rounds_lineage_id_prism_review_lineages_id_fk" FOREIGN KEY ("lineage_id") REFERENCES "public"."prism_review_lineages"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_rounds" ADD CONSTRAINT "prism_review_rounds_base_round_id_prism_review_rounds_id_fk" FOREIGN KEY ("base_round_id") REFERENCES "public"."prism_review_rounds"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_threads" ADD CONSTRAINT "prism_review_threads_lineage_id_prism_review_lineages_id_fk" FOREIGN KEY ("lineage_id") REFERENCES "public"."prism_review_lineages"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_threads" ADD CONSTRAINT "prism_review_threads_born_round_id_prism_review_rounds_id_fk" FOREIGN KEY ("born_round_id") REFERENCES "public"."prism_review_rounds"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_review_threads" ADD CONSTRAINT "prism_review_threads_settled_round_id_prism_review_rounds_id_fk" FOREIGN KEY ("settled_round_id") REFERENCES "public"."prism_review_rounds"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "prism_review_rounds_lineage_id_index" ON "prism_review_rounds" USING btree ("lineage_id");
CREATE INDEX "prism_review_threads_lineage_id_index" ON "prism_review_threads" USING btree ("lineage_id");
CREATE INDEX "prism_review_threads_born_round_id_index" ON "prism_review_threads" USING btree ("born_round_id");
CREATE INDEX "prism_review_threads_settled_round_id_index" ON "prism_review_threads" USING btree ("settled_round_id");
ALTER TABLE "prism_review_threads" DROP COLUMN "born_round";
ALTER TABLE "prism_review_threads" DROP COLUMN "round_id";
ALTER TABLE "prism_review_threads" DROP COLUMN "issue_index";
ALTER TABLE "prism_review_threads" DROP COLUMN "anchors";