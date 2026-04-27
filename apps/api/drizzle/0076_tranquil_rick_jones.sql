CREATE TYPE "public"."_user_device_platform" AS ENUM('IOS', 'ANDROID', 'WEB');
CREATE TABLE "user_devices" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"identifier" text NOT NULL,
	"name" text NOT NULL,
	"platform" "_user_device_platform" NOT NULL,
	"last_active_at" timestamp with time zone DEFAULT now() NOT NULL,
	"last_active_ip" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_devices_user_id_identifier_unique" UNIQUE("user_id","identifier")
);

ALTER TABLE "user_sessions" ADD COLUMN "device_id" text;
ALTER TABLE "user_devices" ADD CONSTRAINT "user_devices_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "user_devices_user_id_index" ON "user_devices" USING btree ("user_id");
ALTER TABLE "user_sessions" ADD CONSTRAINT "user_sessions_device_id_user_devices_id_fk" FOREIGN KEY ("device_id") REFERENCES "public"."user_devices"("id") ON DELETE restrict ON UPDATE cascade;