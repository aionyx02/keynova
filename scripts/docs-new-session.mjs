#!/usr/bin/env node

import { ensureSessionFile, repoPath, todayTaipei } from "./docs-utils.mjs";

const date = process.argv[2] || todayTaipei();
const filePath = ensureSessionFile(date);
console.log(`[docs-new-session] Ready: ${repoPath(filePath)}`);
