-- eval 전면 재설계 데이터 이관. 컷오버 시 1회 실행한다.
-- 전제: 0018 마이그레이션이 적용되어 신 테이블이 존재하고, 구 테이블도 아직 살아 있다.
--
-- id를 보존한다 — runs_v2.id = feedback_sets.id, run_items.id = feedbacks.id, judgments_v2.id
-- = judgments.id. 그래서 진행 중인 라운드의 임시저장이 그대로 살아남는다.
--
-- 구세대 폐기 기준: feedback_sets.review IS NULL인 세트와 그것을 참조하는 라운드·태스크·판정은
-- 옮기지 않는다.

-- 1. 총평 원본을 먼저 원장에 백업한다. 전개(expand-reviews.ts)가 틀려도 여기서 다시 만든다.
INSERT OR IGNORE INTO ledgers (run_id, key, value, created_at)
SELECT id, 'legacy/review', review, unixepoch() FROM feedback_sets WHERE review IS NOT NULL;

-- 2. 문서
INSERT OR IGNORE INTO documents_v2 (id, ref_id, content, character_count, genre, sampling_id, created_at)
SELECT id, ref_id, content, character_count, genre, NULL, created_at FROM documents;

-- 3. 프롬프트 묶음. 세대는 research 키 유무로 1회 판정한다.
INSERT OR IGNORE INTO prompt_sets (id, generation_id, label, note, content, created_at)
SELECT id,
       CASE WHEN json_extract(content, '$.research') IS NOT NULL THEN 'editorial' ELSE 'analysis' END,
       label, note, content, created_at
FROM analysis_prompt_sets;

-- 4. 실행 — 산출물이 있는 세트 기준.
INSERT OR IGNORE INTO runs_v2 (id, document_id, prompt_set_id, instance_id, status, phase, error, created_at, finished_at)
SELECT fs.id,
       fs.document_id,
       COALESCE(
         json_extract(pr.meta, '$.promptSetId'),
         (SELECT aps.id FROM analysis_prompt_sets aps
           JOIN variants v ON v.label = aps.label
          WHERE v.id = pr.variant_id)
       ),
       prd.workflow_instance_id,
       COALESCE(prd.status, 'done'),
       NULL,
       prd.error,
       pr.created_at,
       pr.finished_at
FROM feedback_sets fs
JOIN pipeline_runs pr ON pr.id = fs.run_id
LEFT JOIN pipeline_run_docs prd ON prd.run_id = fs.run_id AND prd.document_id = fs.document_id
WHERE fs.review IS NOT NULL;

-- 4b. 세트가 없는 실패 문서는 run_doc.id를 쓴다 — 실패 기록도 남겨야 재실행 판단이 선다.
INSERT OR IGNORE INTO runs_v2 (id, document_id, prompt_set_id, instance_id, status, phase, error, created_at, finished_at)
SELECT prd.id, prd.document_id,
       json_extract(pr.meta, '$.promptSetId'),
       prd.workflow_instance_id, prd.status, NULL, prd.error, pr.created_at, pr.finished_at
FROM pipeline_run_docs prd
JOIN pipeline_runs pr ON pr.id = prd.run_id
WHERE pr.kind = 'analysis'
  AND json_extract(pr.meta, '$.promptSetId') IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM feedback_sets fs WHERE fs.run_id = prd.run_id AND fs.document_id = prd.document_id);

-- 5. 지적 → 항목. polarity는 신세대 전량 'issue'라 버린다.
INSERT OR IGNORE INTO run_items (id, run_id, kind, ord, body, facets)
SELECT f.id, f.set_id, 'finding', f.ord, f.body,
       json_object('axis', COALESCE(f.category, ''), 'layer', COALESCE(f.layer, ''))
FROM feedbacks f
JOIN feedback_sets fs ON fs.id = f.set_id
WHERE fs.review IS NOT NULL;

-- 6. 앵커
INSERT OR IGNORE INTO item_anchors (id, item_id, ord, start_text, end_text, match_start, match_end, note)
SELECT fa.id, fa.feedback_id, fa.ord, fa.start_text, fa.end_text, fa.match_start, fa.match_end, fa.note
FROM feedback_anchors fa
JOIN feedbacks f ON f.id = fa.feedback_id
JOIN feedback_sets fs ON fs.id = f.set_id
WHERE fs.review IS NOT NULL;

-- 7. 앵커 테이블이 빈 초기 세트는 대표 앵커 컬럼에서 백필한다.
INSERT OR IGNORE INTO item_anchors (id, item_id, ord, start_text, end_text, match_start, match_end, note)
SELECT f.id || '-a0', f.id, 0, f.start_text, f.end_text, f.match_start, f.match_end, NULL
FROM feedbacks f
JOIN feedback_sets fs ON fs.id = f.set_id
WHERE fs.review IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM feedback_anchors fa WHERE fa.feedback_id = f.id);

-- 8. 평가자
INSERT OR IGNORE INTO evaluators (email, evaluating, consented_at)
SELECT email, evaluating, created_at FROM evaluator_consents;

-- 9. 라운드. 신세대 세트를 참조하는 라운드만. 활성은 전부 0으로 두고 컷오버 후 오너가 켠다.
INSERT OR IGNORE INTO rounds_v2 (id, label, evaluation_id, active, config, created_at)
SELECT r.id,
       COALESCE(json_extract(r.config, '$.label'), r.stage || ' ' || substr(r.id, 1, 6)),
       CASE WHEN json_extract(ps.content, '$.research') IS NOT NULL THEN 'editorial/triaxial' ELSE 'analysis/triaxial' END,
       0, r.config, r.created_at
FROM rounds r
JOIN tasks t ON t.round_id = r.id
JOIN feedback_sets fs ON fs.id = json_extract(t.set_ids, '$[0]')
JOIN pipeline_runs pr ON pr.id = fs.run_id
LEFT JOIN analysis_prompt_sets ps ON ps.id = json_extract(pr.meta, '$.promptSetId')
WHERE fs.review IS NOT NULL
GROUP BY r.id;

-- 10. 태스크. set_ids 배열의 첫 원소가 곧 run_id다(절대평가라 세트가 하나였다).
INSERT OR IGNORE INTO tasks_v2 (id, round_id, run_id, created_at)
SELECT t.id, t.round_id, json_extract(t.set_ids, '$[0]'), t.created_at
FROM tasks t
JOIN rounds_v2 r2 ON r2.id = t.round_id
JOIN feedback_sets fs ON fs.id = json_extract(t.set_ids, '$[0]')
WHERE fs.review IS NOT NULL;

-- 11. 반납 기록
INSERT OR IGNORE INTO task_releases (task_id, evaluator_email, created_at)
SELECT rt.task_id, rt.evaluator_email, rt.created_at
FROM released_tasks rt
JOIN tasks_v2 t2 ON t2.id = rt.task_id;

-- 12. 판정. 결과 전체에 대한 답 셋을 payload 하나로 합친다.
--     한 태스크에 판정이 둘 이상이면(구 중복 배정) 가장 이른 것만 옮긴다 — task_id가 unique다.
INSERT OR IGNORE INTO judgments_v2 (id, task_id, evaluator_email, draft, payload, elapsed_seconds, created_at, updated_at)
SELECT j.id, j.task_id, j.evaluator_email, j.draft,
       json_object(
         'readCorrectly', (SELECT rv.read_correctly FROM review_verdicts rv WHERE rv.judgment_id = j.id LIMIT 1),
         'priorityUseful', (SELECT rv.priority_useful FROM review_verdicts rv WHERE rv.judgment_id = j.id LIMIT 1),
         'note', (SELECT rv.note FROM review_verdicts rv WHERE rv.judgment_id = j.id LIMIT 1),
         'helpfulness', json_extract(j.result, '$.scores[0].score'),
         'comment', j.comment
       ),
       COALESCE(j.elapsed_seconds, 0), j.created_at, j.updated_at
FROM judgments j
JOIN tasks_v2 t2 ON t2.id = j.task_id
WHERE j.id = (SELECT j2.id FROM judgments j2 WHERE j2.task_id = j.task_id ORDER BY j2.created_at, j2.id LIMIT 1);

-- 13. 항목별 답
INSERT OR IGNORE INTO judgment_items (id, judgment_id, item_id, payload)
SELECT fv.id, fv.judgment_id, fv.feedback_id,
       json_object('correct', fv.correct, 'needed', fv.needed, 'useful', fv.useful, 'note', fv.note)
FROM feedback_verdicts fv
JOIN judgments_v2 j2 ON j2.id = fv.judgment_id
JOIN run_items ri ON ri.id = fv.feedback_id;

-- 14~15. 원장과 리플레이 캐시.
--
-- stage_cache의 키는 'analysis/{runId}/{documentId}/{subKey}' 형태다. LIKE 연결로 조인하면
-- 6,090 × 392 회 패턴 평가가 일어나 SQLite가 "LIKE or GLOB pattern too complex"로 거부한다
-- (프로덕션 실측). 키를 문자열로 쪼개 등가 조인한다.
--
-- 진단 기록은 남기고 캐시는 비울 수 있어야 하므로 subKey로 가른다.
WITH split AS (
  SELECT sc.key AS key, sc.value AS value, sc.created_at AS created_at,
         substr(rest1, 1, instr(rest1, '/') - 1) AS run_id,
         substr(rest1, instr(rest1, '/') + 1) AS rest2
    FROM (SELECT key, value, created_at, substr(key, 10) AS rest1
            FROM stage_cache WHERE key LIKE 'analysis/%') sc
),
parsed AS (
  SELECT key, value, created_at, run_id,
         substr(rest2, 1, instr(rest2, '/') - 1) AS document_id,
         substr(rest2, instr(rest2, '/') + 1) AS sub_key
    FROM split
   WHERE instr(rest2, '/') > 0
)
INSERT OR IGNORE INTO ledgers (run_id, key, value, created_at)
SELECT fs.id, p.sub_key, p.value, p.created_at
FROM parsed p
JOIN feedback_sets fs ON fs.run_id = p.run_id AND fs.document_id = p.document_id
WHERE fs.review IS NOT NULL
  AND (p.sub_key LIKE 'ledger/%' OR p.sub_key IN ('gates', 'plan', 'execute', 'local', 'research', 'selfcheck/dropped'));

WITH split AS (
  SELECT sc.key AS key, sc.value AS value, sc.created_at AS created_at,
         substr(rest1, 1, instr(rest1, '/') - 1) AS run_id,
         substr(rest1, instr(rest1, '/') + 1) AS rest2
    FROM (SELECT key, value, created_at, substr(key, 10) AS rest1
            FROM stage_cache WHERE key LIKE 'analysis/%') sc
),
parsed AS (
  SELECT key, value, created_at, run_id,
         substr(rest2, 1, instr(rest2, '/') - 1) AS document_id,
         substr(rest2, instr(rest2, '/') + 1) AS sub_key
    FROM split
   WHERE instr(rest2, '/') > 0
)
INSERT OR IGNORE INTO call_cache (run_id, key, value, created_at)
SELECT fs.id, p.sub_key, p.value, p.created_at
FROM parsed p
JOIN feedback_sets fs ON fs.run_id = p.run_id AND fs.document_id = p.document_id
WHERE fs.review IS NOT NULL
  AND NOT (p.sub_key LIKE 'ledger/%' OR p.sub_key IN ('gates', 'plan', 'execute', 'local', 'research', 'selfcheck/dropped'));

-- 16. 단계별 비용. 에디토리얼 구 실행의 'plan' 행은 실은 검수 토큰이다.
INSERT OR IGNORE INTO phase_usage (run_id, phase, calls, prompt_tokens, completion_tokens, cached_tokens, cache_write_tokens)
SELECT fs.id,
       CASE WHEN asu.stage = 'plan' AND json_extract(ps.content, '$.research') IS NOT NULL THEN 'planReview' ELSE asu.stage END,
       asu.calls, asu.prompt_tokens, asu.completion_tokens, asu.cached_tokens, asu.cache_write_tokens
FROM analysis_stage_usage asu
JOIN feedback_sets fs ON fs.run_id = asu.run_id AND fs.document_id = asu.document_id
JOIN pipeline_runs pr ON pr.id = fs.run_id
LEFT JOIN analysis_prompt_sets ps ON ps.id = json_extract(pr.meta, '$.promptSetId')
WHERE fs.review IS NOT NULL;

-- 17. 설정
INSERT OR IGNORE INTO settings_v2 (key, value) SELECT key, value FROM settings;
