ALTER TABLE "user_sessions" ALTER COLUMN "device_id" SET NOT NULL;
ALTER TABLE "user_sessions" ADD CONSTRAINT "user_sessions_user_id_device_id_unique" UNIQUE("user_id","device_id");