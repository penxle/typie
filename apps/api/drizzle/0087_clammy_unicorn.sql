CREATE TABLE "document_bundles" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"seq" integer NOT NULL,
	"epoch" integer DEFAULT 0 NOT NULL,
	"kind" text DEFAULT 'pushed' NOT NULL,
	"payload" "bytea" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_states" ADD COLUMN "heads" "bytea" NOT NULL;
ALTER TABLE "document_states" ADD COLUMN "epoch" integer DEFAULT 0 NOT NULL;
ALTER TABLE "document_states" ADD COLUMN "last_bundle_seq" integer DEFAULT 0 NOT NULL;
ALTER TABLE "document_bundles" ADD CONSTRAINT "document_bundles_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "document_bundles_document_id_seq_index" ON "document_bundles" USING btree ("document_id","seq");
CREATE INDEX "document_bundles_document_id_created_at_index" ON "document_bundles" USING btree ("document_id","created_at");
ALTER TABLE "document_states" DROP COLUMN "graph";