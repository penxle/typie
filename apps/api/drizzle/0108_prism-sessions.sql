CREATE TABLE "prism_sessions" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"prism_agent_id" text NOT NULL,
	"title" text,
	"archived_at" timestamp with time zone,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "prism_sessions_prism_agent_id_unique" UNIQUE("prism_agent_id")
);

ALTER TABLE "prism_sessions" ADD CONSTRAINT "prism_sessions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "prism_sessions_user_id_updated_at_index" ON "prism_sessions" USING btree ("user_id","updated_at");