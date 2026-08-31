CREATE TABLE "prism_document_edits" (
	"id" text PRIMARY KEY NOT NULL,
	"session_id" text NOT NULL,
	"tool_call_id" text NOT NULL,
	"document_id" text NOT NULL,
	"before_heads" "bytea" NOT NULL,
	"after_heads" "bytea" NOT NULL,
	"checkpoint_heads" "bytea" NOT NULL,
	"undone" boolean DEFAULT false NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "prism_document_edits" ADD CONSTRAINT "prism_document_edits_session_id_prism_sessions_id_fk" FOREIGN KEY ("session_id") REFERENCES "public"."prism_sessions"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "prism_document_edits" ADD CONSTRAINT "prism_document_edits_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "prism_document_edits_session_id_tool_call_id_index" ON "prism_document_edits" USING btree ("session_id","tool_call_id");
CREATE INDEX "prism_document_edits_document_id_created_at_index" ON "prism_document_edits" USING btree ("document_id","created_at");