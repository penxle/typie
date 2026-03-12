CREATE TYPE "public"."_note_status" AS ENUM('OPEN', 'RESOLVED');
CREATE TABLE "note_entities" (
	"id" text PRIMARY KEY NOT NULL,
	"note_id" text NOT NULL,
	"entity_id" text NOT NULL,
	CONSTRAINT "note_entities_note_id_entity_id_unique" UNIQUE("note_id","entity_id")
);

ALTER TABLE "notes" ADD COLUMN "site_id" text;
ALTER TABLE "notes" ADD COLUMN "status" "_note_status" DEFAULT 'OPEN' NOT NULL;
ALTER TABLE "note_entities" ADD CONSTRAINT "note_entities_note_id_notes_id_fk" FOREIGN KEY ("note_id") REFERENCES "public"."notes"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "note_entities" ADD CONSTRAINT "note_entities_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "note_entities_entity_id_index" ON "note_entities" USING btree ("entity_id");
ALTER TABLE "notes" ADD CONSTRAINT "notes_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "notes_site_id_state_index" ON "notes" USING btree ("site_id","state");