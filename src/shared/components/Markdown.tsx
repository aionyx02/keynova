import React, { useEffect, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";

interface MarkdownProps {
  content: string;
  className?: string;
}

class MarkdownErrorBoundary extends React.Component<
  { children: React.ReactNode; fallback: React.ReactNode },
  { hasError: boolean }
> {
  constructor(props: { children: React.ReactNode; fallback: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }
  static getDerivedStateFromError() {
    return { hasError: true };
  }
  componentDidCatch() {
    // Swallow render errors and fall back to the plain-text view; the original
    // content is preserved via the `fallback` prop so users still see the model
    // output even if a malformed markdown construct trips react-markdown.
  }
  render() {
    if (this.state.hasError) return this.props.fallback;
    return this.props.children;
  }
}

/**
 * GitHub-flavoured markdown renderer with syntax highlighting. Raw HTML is
 * disabled by default (no `rehype-raw`), so untrusted model output cannot
 * inject `<script>` or event handlers. Failures fall back to the original text
 * inside `<pre>` to preserve content even when react-markdown chokes.
 */
export function Markdown({ content, className }: MarkdownProps) {
  // Lazy-loaded `highlight.js` stylesheet so the panel still works if the asset
  // is missing in a stripped build. Loaded once per session.
  useEffect(() => {
    if (typeof document === "undefined") return;
    const id = "keynova-hljs-theme";
    if (document.getElementById(id)) return;
    import("highlight.js/styles/github-dark.css")
      .catch(() => {
        // Theme is purely cosmetic; ignore failure.
      });
  }, []);

  const [renderError, setRenderError] = useState(false);
  if (renderError) {
    return <pre className={className ?? "whitespace-pre-wrap font-sans"}>{content}</pre>;
  }

  return (
    <MarkdownErrorBoundary
      fallback={<pre className={className ?? "whitespace-pre-wrap font-sans"}>{content}</pre>}
    >
      <div className={className ?? "keynova-md whitespace-pre-wrap break-words text-sm"}>
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          rehypePlugins={[rehypeHighlight]}
          components={{
            code(props) {
              const { className: cls, children, ...rest } = props;
              const isBlock = (cls ?? "").includes("language-");
              if (isBlock) {
                return (
                  <code className={cls} {...rest}>
                    {children}
                  </code>
                );
              }
              return (
                <code className="rounded bg-gray-900/60 px-1 py-0.5 text-[12px]" {...rest}>
                  {children}
                </code>
              );
            },
            pre({ children }) {
              return (
                <pre className="my-2 overflow-x-auto rounded bg-gray-950/60 p-2 text-[12px] leading-relaxed">
                  {children}
                </pre>
              );
            },
            a({ children, href }) {
              return (
                <a
                  href={href}
                  target="_blank"
                  rel="noreferrer"
                  className="text-blue-300 underline hover:text-blue-200"
                >
                  {children}
                </a>
              );
            },
            ul({ children }) {
              return <ul className="my-1 ml-5 list-disc space-y-0.5">{children}</ul>;
            },
            ol({ children }) {
              return <ol className="my-1 ml-5 list-decimal space-y-0.5">{children}</ol>;
            },
            p({ children }) {
              return <p className="my-1 leading-relaxed">{children}</p>;
            },
          }}
        >
          {content}
        </ReactMarkdown>
      </div>
    </MarkdownErrorBoundary>
  );

  // Note: `renderError` is reserved for future imperative error reporting
  // (e.g. if we add a `react-markdown` onError callback later). The setter is
  // referenced here so the linter knows it's intentionally kept.
  void setRenderError;
}
