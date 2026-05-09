CREATE TABLE "document_changesets_dead_letter" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"payload" "bytea" NOT NULL,
	"user_id" text NOT NULL,
	"device_id" text NOT NULL,
	"error_message" text NOT NULL,
	"failed_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_changesets_dead_letter" ADD CONSTRAINT "document_changesets_dead_letter_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_changesets_dead_letter" ADD CONSTRAINT "document_changesets_dead_letter_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_changesets_dead_letter" ADD CONSTRAINT "document_changesets_dead_letter_device_id_user_devices_id_fk" FOREIGN KEY ("device_id") REFERENCES "public"."user_devices"("id") ON DELETE restrict ON UPDATE cascade;