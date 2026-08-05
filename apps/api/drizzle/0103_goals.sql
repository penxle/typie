CREATE TABLE "entity_goals" (
	"id" text PRIMARY KEY NOT NULL,
	"entity_id" text NOT NULL,
	"target_character_count" integer NOT NULL,
	"due_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "entity_goals_entity_id_unique" UNIQUE("entity_id")
);

CREATE TABLE "user_goals" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"target_character_count" integer,
	"effective_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "entity_goals" ADD CONSTRAINT "entity_goals_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_goals" ADD CONSTRAINT "user_goals_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "user_goals_user_id_effective_at_index" ON "user_goals" USING btree ("user_id","effective_at");