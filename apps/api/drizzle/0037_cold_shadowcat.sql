ALTER TYPE "public"."_entity_type" ADD VALUE 'DOCUMENT' BEFORE 'FOLDER';
CREATE TABLE "document_contents" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"snapshot" "bytea" NOT NULL,
	"version" "bytea" NOT NULL,
	"compacted_at" timestamp with time zone DEFAULT now() NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "document_contents_document_id_unique" UNIQUE("document_id")
);

CREATE TABLE "document_version_contributors" (
	"id" text PRIMARY KEY NOT NULL,
	"version_id" text NOT NULL,
	"user_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "document_version_contributors_version_id_user_id_unique" UNIQUE("version_id","user_id")
);

CREATE TABLE "document_versions" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"version" "bytea" NOT NULL,
	"order" integer DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "documents" (
	"id" text PRIMARY KEY NOT NULL,
	"entity_id" text NOT NULL,
	"title" text,
	"subtitle" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "document_contents" ADD CONSTRAINT "document_contents_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_version_contributors" ADD CONSTRAINT "document_version_contributors_version_id_document_versions_id_fk" FOREIGN KEY ("version_id") REFERENCES "public"."document_versions"("id") ON DELETE cascade ON UPDATE cascade;
ALTER TABLE "document_version_contributors" ADD CONSTRAINT "document_version_contributors_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "document_versions" ADD CONSTRAINT "document_versions_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "documents" ADD CONSTRAINT "documents_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "document_versions_document_id_created_at_order_index" ON "document_versions" USING btree ("document_id","created_at","order");
CREATE INDEX "documents_entity_id_index" ON "documents" USING btree ("entity_id");