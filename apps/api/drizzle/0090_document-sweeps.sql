CREATE TABLE "document_sweeps" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"stream_seq" text NOT NULL,
	"zombie_dots" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_sweeps" ADD CONSTRAINT "document_sweeps_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "document_sweeps_document_id_stream_seq_index" ON "document_sweeps" USING btree ("document_id","stream_seq");