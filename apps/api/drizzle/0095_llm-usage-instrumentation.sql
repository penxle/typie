CREATE TYPE "public"."_llm_analysis_run_state" AS ENUM('RUNNING', 'COMPLETED', 'ABORTED', 'FAILED');
CREATE TYPE "public"."_llm_call_state" AS ENUM('COMPLETED', 'FAILED', 'ABORTED');
CREATE TABLE "llm_analysis_runs" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"document_id" text,
	"text_length" integer NOT NULL,
	"chunk_count" integer NOT NULL,
	"prefix_hash" text NOT NULL,
	"full_hash" text NOT NULL,
	"state" "_llm_analysis_run_state" NOT NULL,
	"started_at" timestamp with time zone DEFAULT now() NOT NULL,
	"ended_at" timestamp with time zone
);

CREATE TABLE "llm_call_usage" (
	"id" text PRIMARY KEY NOT NULL,
	"run_id" text NOT NULL,
	"phase" text NOT NULL,
	"model" text NOT NULL,
	"input_tokens" integer,
	"output_tokens" integer,
	"cached_input_tokens" integer,
	"reasoning_tokens" integer,
	"total_tokens" integer,
	"input_chars" integer NOT NULL,
	"duration_ms" integer NOT NULL,
	"cache_status" text,
	"gateway_log_id" text,
	"state" "_llm_call_state" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "llm_analysis_runs" ADD CONSTRAINT "llm_analysis_runs_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "llm_call_usage" ADD CONSTRAINT "llm_call_usage_run_id_llm_analysis_runs_id_fk" FOREIGN KEY ("run_id") REFERENCES "public"."llm_analysis_runs"("id") ON DELETE cascade ON UPDATE cascade;
CREATE INDEX "llm_analysis_runs_user_id_started_at_index" ON "llm_analysis_runs" USING btree ("user_id","started_at");
CREATE INDEX "llm_analysis_runs_user_id_prefix_hash_index" ON "llm_analysis_runs" USING btree ("user_id","prefix_hash");
CREATE INDEX "llm_call_usage_run_id_index" ON "llm_call_usage" USING btree ("run_id");