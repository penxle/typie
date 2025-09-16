CREATE TYPE "public"."_note_state" AS ENUM('ACTIVE', 'DELETED', 'DELETED_CASCADED');
CREATE TABLE "notes" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"entity_id" text,
	"content" text NOT NULL,
	"color" text NOT NULL,
	"order" text NOT NULL,
	"state" "_note_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "notes_user_id_order_unique" UNIQUE NULLS NOT DISTINCT("user_id","order")
);

ALTER TABLE "notes" ADD CONSTRAINT "notes_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "notes" ADD CONSTRAINT "notes_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "notes_user_id_state_order_index" ON "notes" USING btree ("user_id","state","order");
CREATE INDEX "notes_entity_id_state_order_index" ON "notes" USING btree ("entity_id","state","order");