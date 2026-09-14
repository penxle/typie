CREATE TYPE "public"."_publication_state" AS ENUM('SCHEDULED', 'PUBLISHED', 'UNPUBLISHED');
CREATE TYPE "public"."_space_date_display" AS ENUM('NONE', 'PUBLISHED_AT', 'UPDATED_AT');
CREATE TYPE "public"."_space_state" AS ENUM('ACTIVE', 'DELETED');
CREATE TABLE "collections" (
	"id" text PRIMARY KEY NOT NULL,
	"space_id" text NOT NULL,
	"name" text NOT NULL,
	"description" text,
	"cover_id" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "publication_tags" (
	"id" text PRIMARY KEY NOT NULL,
	"publication_id" text NOT NULL,
	"space_id" text NOT NULL,
	"name" text NOT NULL,
	"order" text NOT NULL,
	CONSTRAINT "publication_tags_publication_id_name_unique" UNIQUE("publication_id","name")
);

CREATE TABLE "publication_versions" (
	"id" text PRIMARY KEY NOT NULL,
	"publication_id" text NOT NULL,
	"version" integer NOT NULL,
	"title" text,
	"subtitle" text,
	"graph" "bytea" NOT NULL,
	"text" text NOT NULL,
	"character_count" integer NOT NULL,
	"heads" "bytea" NOT NULL,
	"thumbnail_id" text,
	"excerpt" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "publication_versions_publication_id_version_unique" UNIQUE("publication_id","version")
);

CREATE TABLE "publications" (
	"id" text PRIMARY KEY NOT NULL,
	"document_id" text NOT NULL,
	"space_id" text NOT NULL,
	"state" "_publication_state" NOT NULL,
	"collection_id" text,
	"collection_order" text,
	"pinned_order" text,
	"published_at" timestamp with time zone,
	"scheduled_at" timestamp with time zone,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	"unpublished_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "publications_document_id_unique" UNIQUE("document_id")
);

CREATE TABLE "spaces" (
	"id" text PRIMARY KEY NOT NULL,
	"site_id" text NOT NULL,
	"slug" text NOT NULL,
	"name" text NOT NULL,
	"logo_id" text,
	"description" text,
	"links" jsonb DEFAULT '[]'::jsonb NOT NULL,
	"allow_indexing" boolean DEFAULT true NOT NULL,
	"date_display" "_space_date_display" DEFAULT 'PUBLISHED_AT' NOT NULL,
	"state" "_space_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "collections" ADD CONSTRAINT "collections_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "collections" ADD CONSTRAINT "collections_cover_id_images_id_fk" FOREIGN KEY ("cover_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publication_tags" ADD CONSTRAINT "publication_tags_publication_id_publications_id_fk" FOREIGN KEY ("publication_id") REFERENCES "public"."publications"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publication_tags" ADD CONSTRAINT "publication_tags_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publication_versions" ADD CONSTRAINT "publication_versions_publication_id_publications_id_fk" FOREIGN KEY ("publication_id") REFERENCES "public"."publications"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publication_versions" ADD CONSTRAINT "publication_versions_thumbnail_id_images_id_fk" FOREIGN KEY ("thumbnail_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publications" ADD CONSTRAINT "publications_document_id_documents_id_fk" FOREIGN KEY ("document_id") REFERENCES "public"."documents"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publications" ADD CONSTRAINT "publications_space_id_spaces_id_fk" FOREIGN KEY ("space_id") REFERENCES "public"."spaces"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publications" ADD CONSTRAINT "publications_collection_id_collections_id_fk" FOREIGN KEY ("collection_id") REFERENCES "public"."collections"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "spaces" ADD CONSTRAINT "spaces_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "spaces" ADD CONSTRAINT "spaces_logo_id_images_id_fk" FOREIGN KEY ("logo_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "collections_space_id_index" ON "collections" USING btree ("space_id");
CREATE INDEX "publication_tags_space_id_name_index" ON "publication_tags" USING btree ("space_id","name");
CREATE INDEX "publications_space_id_state_published_at_index" ON "publications" USING btree ("space_id","state","published_at");
CREATE UNIQUE INDEX "publications_space_id_pinned_order_index" ON "publications" USING btree ("space_id","pinned_order") WHERE "publications"."pinned_order" is not null;
CREATE UNIQUE INDEX "publications_collection_id_collection_order_index" ON "publications" USING btree ("collection_id","collection_order") WHERE "publications"."collection_id" is not null;
CREATE INDEX "publications_state_scheduled_at_index" ON "publications" USING btree ("state","scheduled_at") WHERE "publications"."state" = 'SCHEDULED';
CREATE UNIQUE INDEX "spaces_slug_index" ON "spaces" USING btree ("slug");
CREATE INDEX "spaces_site_id_state_index" ON "spaces" USING btree ("site_id","state");