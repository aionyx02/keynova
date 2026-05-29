import { describe, expect, it } from "vitest";

import { looksLikeAiCommandIntent } from "./looksLikeAiCommandIntent";

describe("looksLikeAiCommandIntent", () => {
  it("matches English imperative intents", () => {
    expect(looksLikeAiCommandIntent("list files in this project")).toBe(true);
    expect(looksLikeAiCommandIntent("git push origin HEAD")).toBe(true);
    expect(looksLikeAiCommandIntent("please open the logs folder")).toBe(true);
  });

  it("matches Chinese operation intents", () => {
    expect(looksLikeAiCommandIntent("列出這個專案的檔案")).toBe(true);
    expect(looksLikeAiCommandIntent("開啟設定面板")).toBe(true);
    expect(looksLikeAiCommandIntent("幫我跑 cargo test")).toBe(true);
  });

  it("ignores plain search keywords and questions", () => {
    expect(looksLikeAiCommandIntent("hashmap remove")).toBe(false);
    expect(looksLikeAiCommandIntent("invoice april")).toBe(false);
    expect(looksLikeAiCommandIntent("why is cargo test slow")).toBe(false);
  });

  it("ignores command and terminal sigils", () => {
    expect(looksLikeAiCommandIntent("/help")).toBe(false);
    expect(looksLikeAiCommandIntent("> cargo test")).toBe(false);
  });
});
