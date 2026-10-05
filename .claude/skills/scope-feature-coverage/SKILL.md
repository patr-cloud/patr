---
name: scope-feature-coverage
description: Ask every area agent what a feature needs from their part of Patr, and turn the answers into the plan. Run during planning, before any code is written.
disable-model-invocation: true
argument-hint: "<feature description | plan file | branch or PR> [--only agent,agent]"
---

# Scope feature coverage

Each area agent reports what the feature needs from its part of Patr, including everything nothing in a diff would ever point to. You merge those reports into the plan, so the plan shows every area's work, which agent does it, and in what order.

This is expensive: it runs up to nine agents. The user chose to run it, so run it fully.

## Input

`$ARGUMENTS` is one of:
- a description of the feature;
- the path to a plan file;
- a branch or PR, if the feature is already partly built;
- nothing, meaning the plan currently being discussed.

`--only a,b` limits the run to the agents named. Without it, run all of these:

`api`, `access-control`, `security`, `runners`, `frontend`, `cli`, `iaac`, `platform`, `user-communication`

The `meta` agent is not part of scoping. It runs after execution.

## Steps

1. **Write one brief for all of them.** Say what the feature is and what it adds for users: new resources, operations, fields, principals and behaviours. Include the relevant plan text, or for a branch or PR, a summary of what's built so far. Then tell each agent:
   - it's in **scoping mode**: read-only, and no files edited, backlogs included;
   - its question is *what does this feature need from my area*, not *is this diff correct*.
2. **Launch every selected agent in parallel,** in a single message, one Agent call per area, with `subagent_type` set to the agent's name. Don't do their work yourself while they run.
3. **Merge their reports:**
   - **Nothing needed:** list those agents on one line.
   - **Gaps:** group them by owning agent. Where two agents flagged the same thing, keep it once, under the owner, and note that the other agent agreed.
   - **Handoffs:** gaps one agent spotted in another agent's area go to that area's owner.
   - **Known gaps:** keep the ones agents marked as already in a backlog separate, so it's clear what's new.
   - **Security:** the design requirements the security agent listed become constraints on the plan, not tasks.
   - **Deploy notes:** anything platform says alpha or a self-hosted operator will need.
   - **Questions:** every agent's questions, in one list.
4. **Ask the user the questions.** Their answers decide what's in scope. Don't settle judgement calls on their behalf.
5. **Write the plan's execution section,** following the stage rules in `CLAUDE.md`:
   - a stage table, giving the stage, the agent and the task for each in-scope gap;
   - deferred gaps in the areas that keep a backlog (`DOCS_TODO.md`, `IAAC_TODO.md`, `CLI_TODO.md`), each written as an explicit task for the owning agent to add to its backlog;
   - verification and commits, as the plan needs them.

Stop there. Execution starts when the user approves the plan.
