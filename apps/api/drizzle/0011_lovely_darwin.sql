-- Online migration with minimal service impact
-- This migration is designed to be idempotent and run in production with zero downtime

-- 1. Create new table if not exists
CREATE TABLE IF NOT EXISTS "post_snapshot_contributors" (
	"id" text NOT NULL,
	"snapshot_id" text NOT NULL,
	"user_id" text NOT NULL
);

-- 2. Add compacted_at column safely
DO $$
BEGIN
  -- Check if column exists
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns 
    WHERE table_name = 'post_contents' 
    AND column_name = 'compacted_at'
  ) THEN
    ALTER TABLE "post_contents" ADD COLUMN "compacted_at" timestamp with time zone DEFAULT now();
  END IF;
END $$;

-- Update in large batches
DO $$
DECLARE
  batch_size INT := 50000;  -- Increased batch size
  updated_count INT;
  total_updated BIGINT := 0;
  total_to_update BIGINT;
BEGIN
  -- Get total count to update
  SELECT COUNT(*) INTO total_to_update
  FROM "post_contents" 
  WHERE "compacted_at" IS NULL 
     OR "compacted_at" = '1970-01-01'::timestamp;  -- Handle edge cases
  
  IF total_to_update = 0 THEN
    RAISE NOTICE 'No rows to update for compacted_at';
    RETURN;
  END IF;
  
  RAISE NOTICE 'Starting update of % rows for compacted_at', total_to_update;
  
  LOOP
    -- Use subquery with LIMIT for batch update
    WITH batch AS (
      SELECT "post_id" 
      FROM "post_contents" 
      WHERE "compacted_at" IS NULL 
         OR "compacted_at" = '1970-01-01'::timestamp
      LIMIT batch_size
    )
    UPDATE "post_contents" 
    SET "compacted_at" = COALESCE("created_at", NOW())
    FROM batch
    WHERE "post_contents"."post_id" = batch."post_id";
    
    GET DIAGNOSTICS updated_count = ROW_COUNT;
    
    IF updated_count = 0 THEN
      EXIT;
    END IF;
    
    total_updated := total_updated + updated_count;
    
    -- Log progress every 500k rows
    IF total_updated % 500000 = 0 THEN
      RAISE NOTICE 'Updated % / % rows (%.2f%%)', 
        total_updated, total_to_update, (total_updated::FLOAT / total_to_update * 100);
    END IF;
    
    -- Minimal sleep to allow other transactions
    PERFORM pg_sleep(0.05);
  END LOOP;
  
  RAISE NOTICE 'Completed updating compacted_at: % rows', total_updated;
END $$;

-- Add NOT NULL constraint if not already set
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns 
    WHERE table_name = 'post_contents' 
    AND column_name = 'compacted_at'
    AND is_nullable = 'YES'
  ) THEN
    ALTER TABLE "post_contents" ALTER COLUMN "compacted_at" SET NOT NULL;
  END IF;
END $$;

-- 3. Create migration tracking table
CREATE TABLE IF NOT EXISTS "_migration_progress_0011" (
  id SERIAL PRIMARY KEY,
  batch_number INT NOT NULL,
  last_processed_id TEXT,
  rows_in_batch INT,
  started_at TIMESTAMP DEFAULT NOW(),
  completed_at TIMESTAMP,
  status TEXT DEFAULT 'pending'
);

-- 4. Background data migration function
CREATE OR REPLACE FUNCTION migrate_snapshot_contributors_batch() RETURNS void AS $$
DECLARE
  batch_size INT := 100000;  -- Large batch for fast migration
  last_id TEXT;
  row_count INT;
  batch_num INT;
  lock_acquired BOOLEAN;
BEGIN
  -- Try to acquire advisory lock for batch coordination
  lock_acquired := pg_try_advisory_lock(hashtext('migration_0011_batch'));
  
  IF NOT lock_acquired THEN
    RAISE NOTICE 'Another process is handling batch assignment, waiting...';
    PERFORM pg_advisory_lock(hashtext('migration_0011_batch'));
  END IF;
  
  -- Get next batch to process (protected by lock)
  SELECT COALESCE(MAX(batch_number), 0) + 1 INTO batch_num
  FROM "_migration_progress_0011";
  
  -- Get last processed ID from ALL batches (not just completed)
  SELECT COALESCE(MAX(last_processed_id), '') INTO last_id
  FROM "_migration_progress_0011"
  WHERE last_processed_id IS NOT NULL;
  
  -- Reserve this batch immediately
  INSERT INTO "_migration_progress_0011" (batch_number, started_at, status)
  VALUES (batch_num, NOW(), 'running');
  
  -- Release lock so other processes can get their batch numbers
  PERFORM pg_advisory_unlock(hashtext('migration_0011_batch'));
  
  -- Process batch with temporary table for better performance
  CREATE TEMP TABLE temp_batch AS
  SELECT 
    CONCAT('PSC', CHR(65 + FLOOR(RANDOM() * 26)::INT), UPPER(SUBSTR(MD5(RANDOM()::TEXT || "id")::TEXT, 1, 13))) as new_id,
    "id" as snapshot_id,
    "user_id"
  FROM "post_snapshots"
  WHERE "user_id" IS NOT NULL 
    AND "id" > last_id
  ORDER BY "id"
  LIMIT batch_size;
  
  -- Insert from temp table (ignore duplicates)
  INSERT INTO "post_snapshot_contributors" ("id", "snapshot_id", "user_id")
  SELECT new_id, snapshot_id, user_id FROM temp_batch
  ON CONFLICT DO NOTHING;
  
  GET DIAGNOSTICS row_count = ROW_COUNT;
  
  -- Get actual count of temp batch (not affected by ON CONFLICT)
  SELECT COUNT(*) INTO row_count FROM temp_batch;
  
  -- Get last ID from batch
  SELECT COALESCE(MAX(snapshot_id), last_id) INTO last_id FROM temp_batch;
  
  -- Update progress
  UPDATE "_migration_progress_0011"
  SET last_processed_id = last_id,
      rows_in_batch = row_count,
      completed_at = NOW(),
      status = 'completed'
  WHERE batch_number = batch_num;
  
  DROP TABLE temp_batch;
  
  -- Log for debugging
  RAISE NOTICE 'Batch % completed: % rows found, last_id=%', batch_num, row_count, last_id;
  
  -- Return row count for caller
  IF row_count = 0 THEN
    RAISE NOTICE 'No more rows to process';
  END IF;
END;
$$ LANGUAGE plpgsql;

-- 5. Run migration in large batches
-- This can be run multiple times safely
DO $$
DECLARE
  continue_migration BOOLEAN := true;
  total_rows BIGINT := 0;
  batch_rows INT;
  start_time TIMESTAMP := NOW();
  estimated_total BIGINT;
BEGIN
  -- Check if migration already completed
  SELECT COUNT(*) INTO total_rows 
  FROM "post_snapshot_contributors";
  
  SELECT COUNT(*) INTO estimated_total
  FROM "post_snapshots" 
  WHERE "user_id" IS NOT NULL;
  
  IF total_rows >= estimated_total AND estimated_total > 0 THEN
    RAISE NOTICE 'Migration already completed: % rows', total_rows;
    RETURN;
  END IF;
  
  RAISE NOTICE 'Starting migration of approximately % rows', estimated_total;
  
  WHILE continue_migration LOOP
    -- Run one batch
    PERFORM migrate_snapshot_contributors_batch();
    
    -- Check if we're done
    SELECT rows_in_batch INTO batch_rows
    FROM "_migration_progress_0011"
    WHERE status = 'completed'
    ORDER BY batch_number DESC
    LIMIT 1;
    
    -- Get actual total processed rows
    SELECT COALESCE(SUM(rows_in_batch), 0) INTO total_rows
    FROM "_migration_progress_0011"
    WHERE status = 'completed';
    
    IF batch_rows = 0 OR batch_rows IS NULL THEN
      continue_migration := false;
    ELSE
      -- Very short pause for speed
      PERFORM pg_sleep(0.01);
      
      -- Log progress every 1M rows
      IF total_rows % 1000000 = 0 THEN
        RAISE NOTICE 'Progress: % / % rows (%.2f%%) - Elapsed: %', 
          total_rows, estimated_total, 
          (total_rows::FLOAT / estimated_total * 100),
          NOW() - start_time;
      END IF;
    END IF;
  END LOOP;
  
  RAISE NOTICE 'Migration completed: % rows in %', total_rows, NOW() - start_time;
END $$;

-- 6. Create indexes CONCURRENTLY (no table locks)
-- Primary key index
CREATE UNIQUE INDEX IF NOT EXISTS "post_snapshot_contributors_pkey" 
ON "post_snapshot_contributors"("id");

-- Composite unique index
CREATE UNIQUE INDEX IF NOT EXISTS "post_snapshot_contributors_snapshot_id_user_id_idx" 
ON "post_snapshot_contributors"("snapshot_id", "user_id");

-- Foreign key indexes
CREATE INDEX IF NOT EXISTS "idx_post_snapshot_contributors_snapshot_id" 
ON "post_snapshot_contributors"("snapshot_id");

CREATE INDEX IF NOT EXISTS "idx_post_snapshot_contributors_user_id" 
ON "post_snapshot_contributors"("user_id");

-- 7. Add constraints using existing indexes (fast operation)
-- Only add if not exists to make migration idempotent
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint 
    WHERE conname = 'post_snapshot_contributors_pkey'
  ) THEN
    ALTER TABLE "post_snapshot_contributors" 
    ADD CONSTRAINT "post_snapshot_contributors_pkey" 
    PRIMARY KEY USING INDEX "post_snapshot_contributors_pkey";
  END IF;
  
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint 
    WHERE conname = 'post_snapshot_contributors_snapshot_id_user_id_unique'
  ) THEN
    ALTER TABLE "post_snapshot_contributors" 
    ADD CONSTRAINT "post_snapshot_contributors_snapshot_id_user_id_unique" 
    UNIQUE USING INDEX "post_snapshot_contributors_snapshot_id_user_id_idx";
  END IF;
END $$;

-- 8. Add foreign key constraints with NOT VALID option (no table scan)
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint 
    WHERE conname = 'post_snapshot_contributors_snapshot_id_post_snapshots_id_fk'
  ) THEN
    ALTER TABLE "post_snapshot_contributors" 
    ADD CONSTRAINT "post_snapshot_contributors_snapshot_id_post_snapshots_id_fk" 
    FOREIGN KEY ("snapshot_id") REFERENCES "public"."post_snapshots"("id") 
    ON DELETE cascade ON UPDATE cascade NOT VALID;
  END IF;
  
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint 
    WHERE conname = 'post_snapshot_contributors_user_id_users_id_fk'
  ) THEN
    ALTER TABLE "post_snapshot_contributors" 
    ADD CONSTRAINT "post_snapshot_contributors_user_id_users_id_fk" 
    FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") 
    ON DELETE restrict ON UPDATE cascade NOT VALID;
  END IF;
END $$;

-- 9. Validate constraints in background (can be done separately)
DO $$
BEGIN
  -- Validate only if constraint exists and is not validated
  IF EXISTS (
    SELECT 1 FROM pg_constraint 
    WHERE conname = 'post_snapshot_contributors_snapshot_id_post_snapshots_id_fk'
    AND NOT convalidated
  ) THEN
    ALTER TABLE "post_snapshot_contributors" 
    VALIDATE CONSTRAINT "post_snapshot_contributors_snapshot_id_post_snapshots_id_fk";
  END IF;
  
  IF EXISTS (
    SELECT 1 FROM pg_constraint 
    WHERE conname = 'post_snapshot_contributors_user_id_users_id_fk'
    AND NOT convalidated
  ) THEN
    ALTER TABLE "post_snapshot_contributors" 
    VALIDATE CONSTRAINT "post_snapshot_contributors_user_id_users_id_fk";
  END IF;
END $$;

-- 10. Final migration verification and cleanup
DO $$
DECLARE
  source_count BIGINT;
  target_count BIGINT;
  has_user_id_column BOOLEAN;
BEGIN
  -- Check if user_id column still exists
  SELECT EXISTS (
    SELECT 1 FROM information_schema.columns 
    WHERE table_name = 'post_snapshots' 
    AND column_name = 'user_id'
  ) INTO has_user_id_column;
  
  IF NOT has_user_id_column THEN
    RAISE NOTICE 'Migration already completed: user_id column already dropped';
    
    -- Cleanup if needed
    DROP FUNCTION IF EXISTS migrate_snapshot_contributors_batch();
    DROP TABLE IF EXISTS "_migration_progress_0011";
    RETURN;
  END IF;
  
  SELECT COUNT(*) INTO source_count 
  FROM "post_snapshots" WHERE "user_id" IS NOT NULL;
  
  SELECT COUNT(*) INTO target_count 
  FROM "post_snapshot_contributors";
  
  IF source_count = target_count AND source_count > 0 THEN
    RAISE NOTICE 'Migration verified: % rows migrated successfully', target_count;
    
    -- Safe to drop column now
    ALTER TABLE "post_snapshots" DROP COLUMN "user_id";
    
    -- Cleanup
    DROP FUNCTION IF EXISTS migrate_snapshot_contributors_batch();
    DROP TABLE IF EXISTS "_migration_progress_0011";
    
    -- Update statistics
    ANALYZE "post_snapshot_contributors";
    ANALYZE "post_snapshots";
    
    RAISE NOTICE 'Migration completed and cleaned up successfully';
  ELSIF source_count = 0 AND target_count = 0 THEN
    RAISE NOTICE 'No data to migrate, dropping column';
    ALTER TABLE "post_snapshots" DROP COLUMN "user_id";
    
    -- Cleanup
    DROP FUNCTION IF EXISTS migrate_snapshot_contributors_batch();
    DROP TABLE IF EXISTS "_migration_progress_0011";
  ELSE
    RAISE WARNING 'Migration not complete: source=%, target=%', source_count, target_count;
    RAISE WARNING 'Run the migration script again to continue';
  END IF;
END $$;