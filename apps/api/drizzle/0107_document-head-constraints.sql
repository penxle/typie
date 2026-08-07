DROP INDEX "document_heads_document_id_bucket_index";

UPDATE "document_heads" h
SET "seq" = s.rn + COALESCE(m.max_seq, 0)
FROM (
  SELECT id, document_id, row_number() OVER (PARTITION BY document_id ORDER BY bucket) AS rn
  FROM "document_heads"
  WHERE "seq" IS NULL
) s
JOIN (
  SELECT document_id, MAX("seq") AS max_seq FROM "document_heads" GROUP BY document_id
) m ON m.document_id = s.document_id
WHERE h.id = s.id;

ALTER TABLE "document_heads" ALTER COLUMN "seq" SET NOT NULL;
CREATE UNIQUE INDEX "document_heads_document_id_seq_index" ON "document_heads" USING btree ("document_id","seq");
CREATE INDEX "document_heads_document_id_bucket_index" ON "document_heads" USING btree ("document_id","bucket");