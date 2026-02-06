CREATE TYPE "public"."_document_content_rating" AS ENUM('ALL', 'R15', 'R19');
CREATE TABLE "document_reactions" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"user_id" text,
	"device_id" text NOT NULL,
	"emoji" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "documents" ADD COLUMN "password" text;
ALTER TABLE "documents" ADD COLUMN "content_rating" "_document_content_rating" DEFAULT 'ALL' NOT NULL;
ALTER TABLE "documents" ADD COLUMN "allow_reaction" boolean DEFAULT true NOT NULL;
ALTER TABLE "documents" ADD COLUMN "protect_content" boolean DEFAULT true NOT NULL;
ALTER TABLE "documents" ADD COLUMN "thumbnail_id" text;
ALTER TABLE "document_reactions" ADD CONSTRAINT "document_reactions_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_reactions" ADD CONSTRAINT "document_reactions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "document_reactions_document_id_created_at_index" ON "document_reactions" USING btree ("document_id","created_at");
ALTER TABLE "documents" ADD CONSTRAINT "documents_thumbnail_id_images_id_fk" FOREIGN KEY ("thumbnail_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;