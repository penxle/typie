CREATE TABLE "document_head_contributors" (
	"id" text PRIMARY KEY NOT NULL,
	"head_id" text NOT NULL,
	"user_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "document_head_contributors_head_id_user_id_unique" UNIQUE("head_id","user_id")
);

CREATE TABLE "document_heads" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"bucket" timestamp with time zone NOT NULL,
	"heads" "bytea" NOT NULL,
	"character_count" integer DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_head_contributors" ADD CONSTRAINT "document_head_contributors_head_id_document_heads_id_fk" FOREIGN KEY ("head_id") REFERENCES "public"."document_heads"("id") ON DELETE cascade ON UPDATE cascade;
ALTER TABLE "document_head_contributors" ADD CONSTRAINT "document_head_contributors_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_heads" ADD CONSTRAINT "document_heads_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "document_heads_document_id_bucket_index" ON "document_heads" USING btree ("document_id","bucket");
CREATE INDEX "document_heads_document_id_created_at_index" ON "document_heads" USING btree ("document_id","created_at");