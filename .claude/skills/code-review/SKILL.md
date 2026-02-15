---
description: Code review a pull request and submit as GitHub review (approve/request changes)
---

Provide a code review for the given pull request and submit it as a GitHub review.

**Agent assumptions (applies to all agents and subagents):**

- All tools are functional and will work without error. Do not test tools or make exploratory calls. Make sure this is clear to every subagent that is launched.
- Only call a tool if it is required to complete the task. Every tool call should have a clear purpose.

To do this, follow these steps precisely:

1. Launch a haiku agent to check if any of the following are true:
   - The pull request is closed
   - The pull request is a draft
   - The pull request does not need code review (e.g. automated PR, trivial change that is obviously correct)
   - Claude has already reviewed this PR (check `gh pr view <PR> --json reviews` for reviews left by claude)

   If any condition is true, stop and do not proceed.

   Note: Still review Claude generated PR's.

2. Launch a haiku agent to return a list of file paths (not their contents) for all relevant CLAUDE.md files including:
   - The root CLAUDE.md file, if it exists
   - Any CLAUDE.md files in directories containing files modified by the pull request

3. Launch a sonnet agent to view the pull request and return a summary of the changes

4. Launch 4 agents in parallel to independently review the changes. Each agent should return the list of issues, where each issue includes a description, the file path, line number(s), and the reason it was flagged (e.g. "CLAUDE.md adherence", "bug"). The agents should do the following:

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

5. For each issue found in the previous step by agents 3 and 4, launch parallel subagents to validate the issue. These subagents should get the PR title and description along with a description of the issue. The agent's job is to review the issue to validate that the stated issue is truly an issue with high confidence. For example, if an issue such as "variable is not defined" was flagged, the subagent's job would be to validate that is actually true in the code. Another example would be CLAUDE.md issues. The agent should validate that the CLAUDE.md rule that was violated is scoped for this file and is actually violated. Use Opus subagents for bugs and logic issues, and sonnet agents for CLAUDE.md violations.

6. Filter out any issues that were not validated in step 5. This step will give us our list of high signal issues for our review.

7. Submit a GitHub review:

   **If NO issues were found:**
   Run the following command to approve the PR:

   ```
   gh pr review <PR_NUMBER> --approve --body "No issues found. Checked for bugs and CLAUDE.md compliance."
   ```

   **If issues were found:**
   Submit a review with REQUEST_CHANGES using `gh api`. Construct a JSON payload and submit it in a single API call:

   ```bash
   gh api repos/{owner}/{repo}/pulls/{pull_number}/reviews \
     --method POST \
     -f 'event=REQUEST_CHANGES' \
     -f 'body=Found N issue(s).' \
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

   **IMPORTANT: Only post ONE comment per unique issue. Do not post duplicate comments.**

Use this list when evaluating issues in Steps 4 and 5 (these are false positives, do NOT flag):

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
