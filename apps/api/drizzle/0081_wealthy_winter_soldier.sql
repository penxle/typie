CREATE TABLE "prompts" (
	"id" text PRIMARY KEY NOT NULL,
	"model" text NOT NULL,
	"effort" text,
	"system_prompt" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);
