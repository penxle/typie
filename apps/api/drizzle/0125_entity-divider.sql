ALTER TYPE "public"."_entity_type" ADD VALUE 'DIVIDER';
CREATE TABLE "dividers" (
	"id" text PRIMARY KEY NOT NULL,
	"entity_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "dividers" ADD CONSTRAINT "dividers_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "dividers_entity_id_index" ON "dividers" USING btree ("entity_id");