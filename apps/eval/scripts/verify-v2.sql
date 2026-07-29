-- 이관 검증. 구/신 열이 같아야 하고 마지막 두 질의는 0이어야 한다.
SELECT '실행' AS 항목,
       (SELECT count(*) FROM feedback_sets WHERE review IS NOT NULL) AS 구,
       (SELECT count(*) FROM runs_v2 WHERE id IN (SELECT id FROM feedback_sets WHERE review IS NOT NULL)) AS 신;

SELECT '지적',
       (SELECT count(*) FROM feedbacks f JOIN feedback_sets fs ON fs.id = f.set_id WHERE fs.review IS NOT NULL),
       (SELECT count(*) FROM run_items WHERE kind = 'finding');

SELECT '앵커 보유 지적',
       (SELECT count(*) FROM feedbacks f JOIN feedback_sets fs ON fs.id = f.set_id WHERE fs.review IS NOT NULL),
       (SELECT count(DISTINCT item_id) FROM item_anchors);

SELECT '태스크',
       (SELECT count(*) FROM tasks t JOIN feedback_sets fs ON fs.id = json_extract(t.set_ids, '$[0]') WHERE fs.review IS NOT NULL),
       (SELECT count(*) FROM tasks_v2);

SELECT '판정',
       (SELECT count(DISTINCT j.task_id) FROM judgments j JOIN tasks_v2 t2 ON t2.id = j.task_id),
       (SELECT count(*) FROM judgments_v2);

SELECT '항목별 답',
       (SELECT count(*) FROM feedback_verdicts fv JOIN run_items ri ON ri.id = fv.feedback_id),
       (SELECT count(*) FROM judgment_items);

SELECT '총평 백업',
       (SELECT count(*) FROM feedback_sets WHERE review IS NOT NULL),
       (SELECT count(*) FROM ledgers WHERE key = 'legacy/review');

SELECT '고아 항목별 답' AS 항목, count(*) AS 건수
FROM judgment_items ji LEFT JOIN run_items ri ON ri.id = ji.item_id WHERE ri.id IS NULL;

SELECT '프롬프트 없는 실행', count(*) FROM runs_v2 WHERE prompt_set_id IS NULL;

SELECT '세대 미상 프롬프트 묶음', count(*) FROM prompt_sets WHERE generation_id NOT IN ('editorial', 'analysis');
