CREATE TABLE "document_character_count_changes" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"document_id" text NOT NULL,
	"bucket" timestamp with time zone NOT NULL,
	"additions" integer DEFAULT 0 NOT NULL,
	"deletions" integer DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_character_count_changes" ADD CONSTRAINT "document_character_count_changes_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_character_count_changes" ADD CONSTRAINT "document_character_count_changes_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "document_character_count_changes_user_id_document_id_bucket_index" ON "document_character_count_changes" USING btree ("user_id","document_id","bucket");
CREATE INDEX "document_character_count_changes_user_id_bucket_index" ON "document_character_count_changes" USING btree ("user_id","bucket");