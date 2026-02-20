---
description: Code review a pull request and submit as GitHub review (approve/request changes)
---

Provide a code review for the given pull request and submit it as a GitHub review.

**Agent assumptions (applies to all agents and subagents):**

- All tools are functional and will work without error. Do not test tools or make exploratory calls. Make sure this is clear to every subagent that is launched.
- Only call a tool if it is required to complete the task. Every tool call should have a clear purpose.

To do this, follow these steps precisely:

1. Launch a haiku agent to gather PR state and prior review info. The agent should:

   a. Check if any of the following are true (if so, stop and do not proceed):
   - The pull request is closed
   - The pull request is a draft

   b. Check if Claude has previously reviewed this PR by running:
   `gh pr view <PR> --json reviews,commits`

   Determine:
   - Whether Claude has submitted a prior review (look for reviews left by claude/Claude)
   - If so, the **type** of the last actionable review (`APPROVED` or `CHANGES_REQUESTED` — ignore `COMMENTED` for determining review type)
   - The **commit SHA** that Claude's last review was submitted against (`commit.oid` on the review)
   - The **current HEAD SHA** of the PR branch
   - The **diff checksum** stored as a commit status on the reviewed commit (context: `claude/diff-checksum`, value in `description` field). Query it:

     ```bash
     gh api repos/{owner}/{repo}/commits/{REVIEWED_COMMIT_SHA}/statuses --jq '.[] | select(.context == "claude/diff-checksum") | .description'
     ```

   Compare the reviewed commit SHA to the current HEAD SHA. If they differ, the HEAD has changed — but this does NOT necessarily mean the code has changed. Rebases, base branch updates (e.g., GitHub UI "Update branch" button, Graphite bot), or local rebase + force push can change the HEAD SHA without altering the PR's actual changes.

   To determine if the **code actually changed**, compare the PR diff at the time of the last review vs now. Run:
   `gh pr diff <PR>` and compare it against the diff that was reviewed. Since we cannot retrieve the old diff directly, use a content-based heuristic: compute a checksum of the current PR diff (`gh pr diff <PR> | sha256sum`) and compare it to the checksum stored as a commit status on the reviewed commit. If the checksums match, the code is unchanged despite the HEAD SHA change.

   If no diff checksum status exists on the reviewed commit (e.g., reviews from before this workflow was added), treat any HEAD SHA change as a code change.

   Return one of these states:
   - `no_prior_review` — Claude has never reviewed this PR → proceed with full review
   - `approved_no_change` — Claude approved, code unchanged (same HEAD or same diff checksum) → **stop, do not proceed**
   - `approved_code_changed` — Claude approved, code actually changed → proceed (re-review mode)
   - `changes_requested_no_change` — Claude requested changes, code unchanged, unresolved threads remain → **stop, do not proceed**
   - `changes_requested_all_resolved` — Claude requested changes, code unchanged, but ALL of Claude's review threads are resolved → **skip to step 9D** (approve)
   - `changes_requested_code_changed` — Claude requested changes, code actually changed → proceed (re-review mode: check fixes + new changes)

   To distinguish `changes_requested_no_change` from `changes_requested_all_resolved`, the agent must check thread resolution status via the GraphQL query below when the prior review was `CHANGES_REQUESTED` and the code is unchanged. If every Claude-authored review thread is resolved, return `changes_requested_all_resolved`.

   Also return (if applicable — i.e., for any re-review state where Claude has a prior review):
   - The list of review comment bodies, paths, and lines from Claude's last review (for cross-referencing in step 5 to avoid duplicate flags)

   For `changes_requested_code_changed` specifically, also return the **thread node IDs** for each comment so they can be resolved in step 8. Fetch threads and comments together via GraphQL:

   ```bash
   gh api graphql -f query='query { repository(owner: "<OWNER>", name: "<REPO>") { pullRequest(number: <PR_NUMBER>) { reviewThreads(last: 100) { nodes { id isResolved comments(first: 10) { nodes { body path line author { login } } } } } } } }'
   ```

   Filter threads where the first comment's author is claude/Claude and match them to the prior review.

2. **[Re-review only: `changes_requested_code_changed`]** Launch parallel sonnet agents to check whether each previously flagged issue from Claude's last `CHANGES_REQUESTED` review has been fixed in the current code. Each agent receives:
   - The original review comment (body, path, line)
   - The current state of the relevant file (read the file at the PR's HEAD)

   Each agent should return whether the issue is **fixed** or **still present**.

3. Launch a haiku agent to return a list of file paths (not their contents) for all relevant CLAUDE.md files including:
   - The root CLAUDE.md file, if it exists
   - Any CLAUDE.md files in directories containing files modified by the pull request

4. Launch a sonnet agent to view the pull request and return a summary of the changes

5. Launch 4 agents in parallel to independently review the changes. Each agent should return the list of issues, where each issue includes a description, the file path, line number(s), and the reason it was flagged (e.g. "CLAUDE.md adherence", "bug").

   **Scope note for re-review modes (`approved_code_changed` or `changes_requested_code_changed`):** Review the full PR diff (since the branch may have been amended/force-pushed, there is no reliable way to isolate only "new" changes). However, do NOT re-flag issues that were already raised in a prior review — each agent must receive the list of prior review comments from step 1b and skip any issue that matches an existing comment (compare by file path and issue description, not line numbers which may have shifted).

   The agents should do the following:

   Agents 1 + 2: CLAUDE.md compliance sonnet agents
   Audit changes for CLAUDE.md compliance in parallel. Note: When evaluating CLAUDE.md compliance for a file, you should only consider CLAUDE.md files that share a file path with the file or parents.

   Agent 3: Opus bug agent (parallel subagent with agent 4)
   Scan for obvious bugs. Focus only on the diff itself without reading extra context. Flag only significant bugs; ignore nitpicks and likely false positives. Do not flag issues that you cannot validate without looking at context outside of the git diff.

   Agent 4: Opus bug agent (parallel subagent with agent 3)
   Look for problems that exist in the introduced code. This could be security issues, incorrect logic, etc. Only look for issues that fall within the changed code.

   **CRITICAL: We only want HIGH SIGNAL issues.** Flag issues where:
   - The code will fail to compile or parse (syntax errors, type errors, missing imports, unresolved references)
   - The code will definitely produce wrong results regardless of inputs (clear logic errors)
   - Clear, unambiguous CLAUDE.md violations where you can quote the exact rule being broken

   Do NOT flag:
   - Code style or quality concerns
   - Potential issues that depend on specific inputs or state
   - Subjective suggestions or improvements

   If you are not certain an issue is real, do not flag it. False positives erode trust and waste reviewer time.

   In addition to the above, each subagent should be told the PR title and description. This will help provide context regarding the author's intent.

6. For each issue found in the previous step by agents 3 and 4, launch parallel subagents to validate the issue. These subagents should get the PR title and description along with a description of the issue. The agent's job is to review the issue to validate that the stated issue is truly an issue with high confidence. For example, if an issue such as "variable is not defined" was flagged, the subagent's job would be to validate that is actually true in the code. Another example would be CLAUDE.md issues. The agent should validate that the CLAUDE.md rule that was violated is scoped for this file and is actually violated. Use Opus subagents for bugs and logic issues, and sonnet agents for CLAUDE.md violations.

7. Filter out any issues that were not validated in step 6. This step will give us our list of high signal issues for our review.

8. **[Re-review only: `changes_requested_code_changed`]** Resolve fixed review threads. For each issue from step 2 that was determined to be **fixed**, resolve its review thread using the GraphQL API and the thread node IDs returned from step 1b:

   ```bash
   gh api graphql -f query='mutation { resolveReviewThread(input: { threadId: "<THREAD_NODE_ID>" }) { thread { id isResolved } } }'
   ```

   Only resolve threads that correspond to issues confirmed as fixed in step 2.

9. Submit a GitHub review. The action depends on the review state from step 1 and the issues found:

   **A. First review (`no_prior_review`):**
   - No issues → APPROVE
   - Issues found → REQUEST_CHANGES

   **B. Re-review after prior APPROVE (`approved_code_changed`):**
   - No new issues → do NOT submit a review. Just update the diff checksum (see below). This prevents stale checksums from triggering infinite re-reviews.
   - New issues found → REQUEST_CHANGES

   **C. Re-review after prior REQUEST_CHANGES (`changes_requested_code_changed`):**
   - All prior issues fixed AND no new issues → APPROVE
   - Some prior issues still present, no new issues → do NOT submit a review. Just update the diff checksum. The existing REQUEST_CHANGES and its unresolved threads are sufficient.
   - New issues found (regardless of prior issue status) → REQUEST_CHANGES (only for NEW issues not already flagged; do not re-post comments for unresolved prior issues)

   **D. All threads resolved without code change (`changes_requested_all_resolved`):**
   - APPROVE. All previously flagged issues have been resolved by the author (e.g., false positives acknowledged, issues addressed outside of the PR diff). No re-review of the diff is needed.

   **Diff checksum**: After every step 9 action (including no-ops in 9B/9C), compute the current PR diff checksum and store it as a **commit status** on the current HEAD SHA. This is invisible in the PR UI and is used by future re-reviews (step 1b) to detect whether the code actually changed despite HEAD SHA changes (e.g., rebases).

   ```bash
   CHECKSUM=$(gh pr diff <PR_NUMBER> | sha256sum | awk '{print $1}')
   HEAD_SHA=$(gh pr view <PR_NUMBER> --json headRefOid -q .headRefOid)
   gh api repos/{owner}/{repo}/statuses/$HEAD_SHA \
     -f state=success \
     -f context=claude/diff-checksum \
     -f "description=$CHECKSUM"
   ```

   **To APPROVE:**

   ```
   gh pr review <PR_NUMBER> --approve
   ```

   **To REQUEST_CHANGES:**
   Submit a review with REQUEST_CHANGES using `gh api`. Construct a JSON payload and submit it in a single API call:

   ```bash
   gh api repos/{owner}/{repo}/pulls/{pull_number}/reviews \
     --method POST \
     -f 'event=REQUEST_CHANGES' \
     --input <(echo '{
       "comments": [
         {
           "path": "relative/path/to/file.ts",
           "line": 42,
           "side": "RIGHT",
           "body": "Description of the issue"
         }
       ]
     }')
   ```

   For each comment:
   - `path`: The file path relative to the repository root
   - `line`: The line number in the new version of the file
   - `side`: Always `RIGHT` (we comment on the new code)
   - `body`: A brief description of the issue. For small, self-contained fixes, include a GitHub suggestion block. For larger fixes, describe the issue and suggested fix without a suggestion block. Never include a suggestion block unless committing the suggestion fixes the issue entirely.
   - For multi-line comments, add `start_line` and `start_side: RIGHT` fields

   **IMPORTANT: Only post ONE comment per unique issue. Do not post duplicate comments. Do not re-post comments for issues already flagged in a prior review.**

Use this list when evaluating issues in Steps 5 and 6 (these are false positives, do NOT flag):

- Pre-existing issues
- Something that appears to be a bug but is actually correct
- Pedantic nitpicks that a senior engineer would not flag
- Issues that a linter will catch (do not run the linter to verify)
- General code quality concerns (e.g., lack of test coverage, general security issues) unless explicitly required in CLAUDE.md
- Issues mentioned in CLAUDE.md but explicitly silenced in the code (e.g., via a lint ignore comment)

Notes:

- Use gh CLI to interact with GitHub (e.g., fetch pull requests, view diffs). Do not use web fetch.
- Create a todo list before starting.
- You must cite and link each issue in review comments (e.g., if referring to a CLAUDE.md, include a link to it).
- When linking to code in review comments, follow this format precisely: https://github.com/owner/repo/blob/FULL_SHA/path/file.ext#L10-L15
  - Requires full git sha (not abbreviated)
  - Repo name must match the repo you're reviewing
  - Line range format is L[start]-L[end]
  - Provide at least 1 line of context before and after
