-- 총평 전개의 입력. migrate-v2.sql이 백업한 원본과, 그 실행의 지적 id를 ord 순으로 낸다.
-- feedbackIndexes가 이 순번을 가리키므로 순서가 곧 정합성이다.
SELECT l.run_id AS runId,
       l.value AS review,
       (SELECT json_group_array(id)
          FROM (SELECT ri.id AS id
                  FROM run_items ri
                 WHERE ri.run_id = l.run_id AND ri.kind = 'finding'
                 ORDER BY ri.ord)) AS findingIds
FROM ledgers l
WHERE l.key = 'legacy/review';
