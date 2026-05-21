#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { JSDOM, VirtualConsole } from "jsdom";
import { ROOT, repoPath } from "./docs-utils.mjs";

const TASKS_STATE = "docs/state/tasks.json";
const SUMMARY_STATE = "docs/state/tasks-summary.json";
const SUGGESTIONS_STATE = "docs/state/workbench-suggestions.json";
const WORKBENCH_HTML = "docs/workbench/tasks.html";

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

  if (summary.authority?.full_state !== TASKS_STATE) {
    errors.push(`${SUMMARY_STATE}: authority.full_state must point to ${TASKS_STATE}.`);
  }
  if (summary.authority?.workbench !== WORKBENCH_HTML) {
    errors.push(`${SUMMARY_STATE}: authority.workbench must point to ${WORKBENCH_HTML}.`);
  }
  if (summary.workflow?.current_task !== state.workflow?.current_task) {
    errors.push(`${SUMMARY_STATE}: workflow.current_task does not match ${TASKS_STATE}.`);
  }
  if ((summary.tasks || []).length !== (state.tasks || []).length) {
    errors.push(`${SUMMARY_STATE}: task count does not match ${TASKS_STATE}.`);
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
const suggestionsState = readJson(SUGGESTIONS_STATE);

validateSuggestions(suggestionsState);
validateTaskState(tasksState, summaryState);
await validateHtml(tasksState, suggestionsState);

if (errors.length > 0) {
  console.error("[docs-workbench-guard] Workbench checks failed:");
  for (const error of errors) {
    console.error(`  - ${error}`);
  }
  process.exit(1);
}

console.log(`[docs-workbench-guard] Workbench checks passed for ${repoPath(path.join(ROOT, WORKBENCH_HTML))}.`);
