// Vitest setup — runs once per test worker before any test file loads.

// jsdom does not implement `Element.scrollIntoView`; SearchResultsList calls
// it on each focused row ref. Polyfill as a no-op so component tests can
// mount the list without crashing.
if (typeof Element !== "undefined" && !("scrollIntoView" in Element.prototype)) {
  Object.defineProperty(Element.prototype, "scrollIntoView", {
    value: () => {},
    writable: true,
  });
}
