import { describe, it, expect } from "vitest";

import { classifyNlIntent } from "./classifyNlIntent";

describe("classifyNlIntent", () => {
  describe("fix", () => {
    it("routes cargo error code shape to fix", () => {
      expect(classifyNlIntent("error[E0308]: mismatched types")).toEqual({
        id: "fix",
        text: "error[E0308]: mismatched types",
      });
    });

    it("routes Rust panic to fix", () => {
      expect(classifyNlIntent("thread 'main' panicked at src/x.rs:1")).toEqual({
        id: "fix",
        text: "thread 'main' panicked at src/x.rs:1",
      });
    });

    it("routes python TypeError to fix", () => {
      expect(classifyNlIntent("TypeError: cannot read property foo of undefined")?.id).toBe("fix");
    });

    it("routes fix verb to fix", () => {
      expect(classifyNlIntent("fix this segfault")).toEqual({
        id: "fix",
        text: "fix this segfault",
      });
    });

    it("routes why-does-fail to fix", () => {
      expect(classifyNlIntent("why does cargo build fail")?.id).toBe("fix");
    });

    it("routes CJK fix verbs to fix", () => {
      expect(classifyNlIntent("修復這個錯誤")?.id).toBe("fix");
      expect(classifyNlIntent("為什麼 cargo 一直報錯")?.id).toBe("fix");
    });
  });

  describe("summarize", () => {
    it("routes summarize verb to summarize", () => {
      expect(classifyNlIntent("summarize this article")).toEqual({
        id: "summarize",
        text: "summarize this article",
      });
    });

    it("routes tldr to summarize", () => {
      expect(classifyNlIntent("tldr the rust book chapter 4")?.id).toBe("summarize");
      expect(classifyNlIntent("tl;dr release notes")?.id).toBe("summarize");
    });

    it("routes CJK summarize cues", () => {
      expect(classifyNlIntent("總結這份文件")?.id).toBe("summarize");
      expect(classifyNlIntent("摘要這段對話")?.id).toBe("summarize");
    });
  });

  describe("explain", () => {
    it("routes what-is to explain", () => {
      expect(classifyNlIntent("what is a rust hashmap")).toEqual({
        id: "explain",
        text: "what is a rust hashmap",
      });
    });

    it("routes explain verb to explain", () => {
      expect(classifyNlIntent("explain how futures work")?.id).toBe("explain");
    });

    it("routes describe / define to explain", () => {
      expect(classifyNlIntent("describe the borrow checker")?.id).toBe("explain");
      expect(classifyNlIntent("define monad")?.id).toBe("explain");
    });

    it("routes CJK explain cues", () => {
      expect(classifyNlIntent("解釋一下 borrow checker")?.id).toBe("explain");
      expect(classifyNlIntent("什麼是 ownership")?.id).toBe("explain");
    });

    it("prefers fix over explain when both look applicable", () => {
      // "why does X fail" matches both EXPLAIN why-clause and FIX why-clause.
      // Fix should win because the user has a concrete failure to debug.
      expect(classifyNlIntent("why does my cargo test fail")?.id).toBe("fix");
    });
  });

  describe("cmd", () => {
    it("routes list-files style intents to cmd", () => {
      expect(classifyNlIntent("list files in this project")).toEqual({
        id: "cmd",
        text: "list files in this project",
      });
    });

    it("routes shell-tool-first intents to cmd", () => {
      expect(classifyNlIntent("git push current branch")?.id).toBe("cmd");
    });

    it("routes CJK action-verb + object to cmd", () => {
      expect(classifyNlIntent("列出專案的檔案")?.id).toBe("cmd");
    });
  });

  describe("null", () => {
    it("returns null for empty and whitespace-only queries", () => {
      expect(classifyNlIntent("")).toBeNull();
      expect(classifyNlIntent("   ")).toBeNull();
    });

    it("returns null for terminal / command sigils", () => {
      expect(classifyNlIntent("/help")).toBeNull();
      expect(classifyNlIntent("> ls")).toBeNull();
    });

    it("returns null for arbitrary noun-only search queries", () => {
      expect(classifyNlIntent("hashmap")).toBeNull();
      expect(classifyNlIntent("README.md")).toBeNull();
    });
  });
});