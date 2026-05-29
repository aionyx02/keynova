// PERF.1 — Lazy Markdown entry. The actual react-markdown + remark-gfm +
// rehype-highlight stack lives in `MarkdownImpl.tsx` and is only fetched on
// first use, so the markdown chunk stays out of the main bundle until a
// capability answer streams. Consumers do not change: they still
// `import { Markdown } from ".../Markdown"`.
//
// During the brief load of the impl chunk we render the raw content in a
// `<pre>` so the streaming answer card still shows text instead of blanking.
// On subsequent renders within the same session the chunk is cached and the
// fallback never appears.

import React, { Suspense } from "react";

interface MarkdownProps {
  content: string;
  className?: string;
}

const MarkdownImpl = React.lazy(() =>
  import("./MarkdownImpl").then((mod) => ({ default: mod.Markdown })),
);

function MarkdownFallback({ content, className }: MarkdownProps) {
  return (
    <pre className={className ?? "whitespace-pre-wrap font-sans"}>{content}</pre>
  );
}

export function Markdown(props: MarkdownProps) {
  return (
    <Suspense fallback={<MarkdownFallback {...props} />}>
      <MarkdownImpl {...props} />
    </Suspense>
  );
}
