#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { JSDOM, VirtualConsole } from "jsdom";
import { ROOT, repoPath } from "./docs-utils.mjs";

const TASKS_STATE = "docs/state/tasks.json";
const SUMMARY_STATE = "docs/state/tasks-summary.json";
const DECISION_SUMMARY_STATE = "docs/state/decision-summary.json";
const SUGGESTIONS_STATE = "docs/state/workbench-suggestions.json";
const WORKBENCH_HTML = "docs/workbench/tasks.html";
const MAX_TASKS_SUMMARY_BYTES = 6_000;
const MAX_DECISION_SUMMARY_BYTES = 3_000;

const errors = [];

function readJson(relPath) {
  try {
    return JSON.parse(fs.readFileSync(path.join(ROOT, relPath), "utf8"));
  } catch (error) {
    errors.push(`Unable to parse ${relPath}: ${error.message}`);
    return null;
  }
}

function readText(relPath) {
  try {
    return fs.readFileSync(path.join(ROOT, relPath), "utf8");
  } catch (error) {
    errors.push(`Unable to read ${relPath}: ${error.message}`);
    return "";
  }
}

function fileSize(relPath) {
  try {
    return fs.statSync(path.join(ROOT, relPath)).size;
  } catch (error) {
    errors.push(`Unable to stat ${relPath}: ${error.message}`);
    return 0;
  }
}

function requireUniqueIds(items, relPath, label) {
  const seen = new Set();
  for (const item of items || []) {
    if (!item?.id) {
      errors.push(`${relPath}: ${label} item is missing id.`);
      continue;
    }
    if (seen.has(item.id)) {
      errors.push(`${relPath}: duplicate ${label} id "${item.id}".`);
    }
    seen.add(item.id);
  }
}

function validateSuggestions(suggestions) {
  if (!suggestions) return;
  const questions = Array.isArray(suggestions.questions) ? suggestions.questions : [];
  requireUniqueIds(questions, SUGGESTIONS_STATE, "question");

  for (const question of questions) {
    const options = Array.isArray(question.options) ? question.options : [];
    if (options.length === 0) {
      errors.push(`${SUGGESTIONS_STATE}: question "${question.id}" has no options.`);
      continue;
    }

    requireUniqueIds(options, SUGGESTIONS_STATE, `option in ${question.id}`);
    const optionIds = new Set(options.map((option) => option.id));

    if (question.type === "multiple") {
      const defaults = Array.isArray(question.default) ? question.default : [];
      for (const defaultId of defaults) {
        if (!optionIds.has(defaultId)) {
          errors.push(`${SUGGESTIONS_STATE}: question "${question.id}" default "${defaultId}" is not an option.`);
        }
      }
    } else if (question.default && !optionIds.has(question.default)) {
      errors.push(`${SUGGESTIONS_STATE}: question "${question.id}" default "${question.default}" is not an option.`);
    }
  }
}

function validateTaskState(state, summary) {
  if (!state || !summary) return;
  requireUniqueIds(state.tasks, TASKS_STATE, "task");
  const expectedOpenTasks = state.tasks.filter((task) => task.status !== "done");
  const expectedCompletedTaskIds = state.tasks.filter((task) => task.status === "done").map((task) => task.id);

  if (summary.authority?.full_state !== TASKS_STATE) {
    errors.push(`${SUMMARY_STATE}: authority.full_state must point to ${TASKS_STATE}.`);
  }
  if (summary.authority?.decision_summary !== DECISION_SUMMARY_STATE) {
    errors.push(`${SUMMARY_STATE}: authority.decision_summary must point to ${DECISION_SUMMARY_STATE}.`);
  }
  if (summary.usage?.keep_compact !== true) {
    errors.push(`${SUMMARY_STATE}: usage.keep_compact must be true.`);
  }
  if (summary.workflow?.current_task !== state.workflow?.current_task) {
    errors.push(`${SUMMARY_STATE}: workflow.current_task does not match ${TASKS_STATE}.`);
  }
  if (summary.decision_gate?.source !== DECISION_SUMMARY_STATE) {
    errors.push(`${SUMMARY_STATE}: decision_gate.source must point to ${DECISION_SUMMARY_STATE}.`);
  }
  if ((summary.open_tasks || []).length !== expectedOpenTasks.length) {
    errors.push(`${SUMMARY_STATE}: open task count does not match ${TASKS_STATE}.`);
  }
  if ((summary.completed_task_ids || []).length !== expectedCompletedTaskIds.length) {
    errors.push(`${SUMMARY_STATE}: completed task count does not match ${TASKS_STATE}.`);
  }
  if (fileSize(SUMMARY_STATE) > MAX_TASKS_SUMMARY_BYTES) {
    errors.push(`${SUMMARY_STATE}: must stay under ${MAX_TASKS_SUMMARY_BYTES} bytes to remain a low-token snapshot.`);
  }
}

function validateDecisionSummary(state, decisionSummary) {
  if (!state || !decisionSummary) return;

  if (decisionSummary.authority?.full_state !== TASKS_STATE) {
    errors.push(`${DECISION_SUMMARY_STATE}: authority.full_state must point to ${TASKS_STATE}.`);
  }
  if (decisionSummary.authority?.summary !== SUMMARY_STATE) {
    errors.push(`${DECISION_SUMMARY_STATE}: authority.summary must point to ${SUMMARY_STATE}.`);
  }
  if (decisionSummary.authority?.workbench !== WORKBENCH_HTML) {
    errors.push(`${DECISION_SUMMARY_STATE}: authority.workbench must point to ${WORKBENCH_HTML}.`);
  }
  if (decisionSummary.usage?.prefer_this_file !== true) {
    errors.push(`${DECISION_SUMMARY_STATE}: usage.prefer_this_file must be true.`);
  }
  if (decisionSummary.usage?.avoid_html_by_default !== true) {
    errors.push(`${DECISION_SUMMARY_STATE}: usage.avoid_html_by_default must be true.`);
  }
  if (decisionSummary.decision_reminder?.required !== true) {
    errors.push(`${DECISION_SUMMARY_STATE}: decision_reminder.required must be true.`);
  }
  if (decisionSummary.decision_reminder?.block_implementation_without_confirmation !== true) {
    errors.push(`${DECISION_SUMMARY_STATE}: decision_reminder.block_implementation_without_confirmation must be true.`);
  }
  if (decisionSummary.workflow?.current_task !== state.workflow?.current_task) {
    errors.push(`${DECISION_SUMMARY_STATE}: workflow.current_task does not match ${TASKS_STATE}.`);
  }
  if (fileSize(DECISION_SUMMARY_STATE) > MAX_DECISION_SUMMARY_BYTES) {
    errors.push(`${DECISION_SUMMARY_STATE}: must stay under ${MAX_DECISION_SUMMARY_BYTES} bytes to remain a low-token gate summary.`);
  }
}

async function validateHtml(state, suggestions) {
  if (!state || !suggestions) return;
  const html = readText(WORKBENCH_HTML);
  if (!html.includes('id="workbench-data"')) {
    errors.push(`${WORKBENCH_HTML}: missing workbench-data payload.`);
    return;
  }

  const browserErrors = [];
  const virtualConsole = new VirtualConsole();
  virtualConsole.on("jsdomError", (error) => browserErrors.push(error.message));
  virtualConsole.on("error", (message) => browserErrors.push(String(message)));

  const dom = new JSDOM(html, {
    runScripts: "dangerously",
    resources: "usable",
    url: "http://localhost/docs/workbench/tasks.html",
    virtualConsole,
  });

  await new Promise((resolve) => {
    if (dom.window.document.readyState === "complete") {
      resolve();
      return;
    }
    dom.window.addEventListener("load", resolve, { once: true });
  });

  if (browserErrors.length > 0) {
    errors.push(`${WORKBENCH_HTML}: browser runtime errors: ${browserErrors.join("; ")}`);
  }

  const document = dom.window.document;
  const outcomeCount = document.querySelectorAll("#outcomes .outcome").length;
  const questionCount = document.querySelectorAll("#questionCards .question-card").length;
  const taskCount = document.querySelectorAll("#flow .task").length;

  if (outcomeCount === 0) {
    errors.push(`${WORKBENCH_HTML}: expected at least one outcome card.`);
  }
  const expectedQuestionCount = Array.isArray(suggestions.questions) ? suggestions.questions.length : 0;
  if (questionCount !== expectedQuestionCount) {
    errors.push(`${WORKBENCH_HTML}: expected ${expectedQuestionCount} question cards, found ${questionCount}.`);
  }
  if (taskCount !== state.tasks.length) {
    errors.push(`${WORKBENCH_HTML}: expected ${state.tasks.length} task cards, found ${taskCount}.`);
  }

  const proposal = document.getElementById("proposal");
  if (!proposal?.value) {
    errors.push(`${WORKBENCH_HTML}: proposal textarea is empty.`);
    return;
  }

  let proposalPayload;
  try {
    proposalPayload = JSON.parse(proposal.value);
  } catch (error) {
    errors.push(`${WORKBENCH_HTML}: proposal textarea is not valid JSON: ${error.message}`);
    return;
  }

  if (proposalPayload.decision_status !== "unconfirmed_do_not_execute") {
    errors.push(`${WORKBENCH_HTML}: initial decision_status must be unconfirmed_do_not_execute.`);
  }

  const moveDown = document.querySelector('#flow [data-move="down"]');
  if (!moveDown) {
    errors.push(`${WORKBENCH_HTML}: task cards are missing move controls.`);
  } else {
    moveDown.click();
    const movedPayload = JSON.parse(proposal.value);
    if (movedPayload.decision_status !== "unconfirmed_do_not_execute") {
      errors.push(`${WORKBENCH_HTML}: moving a task must not auto-confirm the decision.`);
    }
    if (movedPayload.order_changed !== true || !Array.isArray(movedPayload.order_diff) || movedPayload.order_diff.length === 0) {
      errors.push(`${WORKBENCH_HTML}: moving a task did not update order_changed/order_diff.`);
    }
  }

  document.getElementById("confirmDecision")?.click();
  const confirmedPayload = JSON.parse(proposal.value);
  if (confirmedPayload.decision_status !== "confirmed") {
    errors.push(`${WORKBENCH_HTML}: confirm button did not switch decision_status to confirmed.`);
  }
  if (!confirmedPayload.confirmed_decision) {
    errors.push(`${WORKBENCH_HTML}: confirmed proposal is missing confirmed_decision.`);
  }

  dom.window.close();
}

const tasksState = readJson(TASKS_STATE);
const summaryState = readJson(SUMMARY_STATE);
const decisionSummaryState = readJson(DECISION_SUMMARY_STATE);
const suggestionsState = readJson(SUGGESTIONS_STATE);

validateSuggestions(suggestionsState);
validateTaskState(tasksState, summaryState);
validateDecisionSummary(tasksState, decisionSummaryState);
await validateHtml(tasksState, suggestionsState);

if (errors.length > 0) {
  console.error("[docs-workbench-guard] Workbench checks failed:");
  for (const error of errors) {
    console.error(`  - ${error}`);
  }
  process.exit(1);
}

console.log(`[docs-workbench-guard] Workbench checks passed for ${repoPath(path.join(ROOT, WORKBENCH_HTML))}.`);
