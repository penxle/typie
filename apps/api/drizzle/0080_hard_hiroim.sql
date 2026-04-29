CREATE TABLE "document_objects" (
	"id" text PRIMARY KEY NOT NULL,
	"hash" text NOT NULL,
	"content" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_commits" ALTER COLUMN "steps" DROP NOT NULL;
ALTER TABLE "document_commits" ALTER COLUMN "device_id" DROP NOT NULL;
ALTER TABLE "document_commits" ADD COLUMN "sequence" bigint NOT NULL GENERATED ALWAYS AS IDENTITY (sequence name "document_commits_sequence_seq" INCREMENT BY 1 MINVALUE 1 MAXVALUE 9223372036854775807 START WITH 1 CACHE 1);
ALTER TABLE "document_commits" ADD COLUMN "object_id" text NOT NULL;
ALTER TABLE "documents" ADD COLUMN "dirty_at" timestamp with time zone;
CREATE UNIQUE INDEX "document_objects_hash_index" ON "document_objects" USING btree ("hash");
ALTER TABLE "document_commits" ADD CONSTRAINT "document_commits_object_id_document_objects_id_fk" FOREIGN KEY ("object_id") REFERENCES "public"."document_objects"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_commits" ADD CONSTRAINT "document_commits_device_id_user_devices_id_fk" FOREIGN KEY ("device_id") REFERENCES "public"."user_devices"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "document_commits_document_id_sequence_index" ON "document_commits" USING btree ("document_id","sequence");
CREATE INDEX "documents_dirty_at_index" ON "documents" USING btree ("dirty_at") WHERE dirty_at IS NOT NULL;