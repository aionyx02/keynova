import type { SVGProps } from "react";

export type UiIconName =
  | "app"
  | "arrow-right"
  | "command"
  | "database"
  | "file"
  | "filter"
  | "folder"
  | "history"
  | "model"
  | "note"
  | "search"
  | "settings"
  | "workspace"
  | "x";

interface Props extends SVGProps<SVGSVGElement> {
  name: UiIconName;
}

function paths(name: UiIconName) {
  switch (name) {
    case "search":
      return (
        <path
          d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      );
    case "command":
      return (
        <>
          <path d="M4.75 6.75h14.5a1.5 1.5 0 0 1 1.5 1.5v7.5a1.5 1.5 0 0 1-1.5 1.5H4.75a1.5 1.5 0 0 1-1.5-1.5v-7.5a1.5 1.5 0 0 1 1.5-1.5Z" />
          <path
            d="m7.75 10.25 2.5 1.75-2.5 1.75M12.75 13.75h3.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </>
      );
    case "database":
      return (
        <>
          <ellipse cx="12" cy="6" rx="6.75" ry="2.75" />
          <path d="M5.25 6v4c0 1.52 3.02 2.75 6.75 2.75s6.75-1.23 6.75-2.75V6" />
          <path d="M5.25 10v4c0 1.52 3.02 2.75 6.75 2.75s6.75-1.23 6.75-2.75v-4" />
          <path d="M5.25 14v4c0 1.52 3.02 2.75 6.75 2.75s6.75-1.23 6.75-2.75v-4" />
        </>
      );
    case "file":
      return (
        <>
          <path d="M8 3.75h5.5L18.25 8.5V19A1.25 1.25 0 0 1 17 20.25H8A1.25 1.25 0 0 1 6.75 19V5A1.25 1.25 0 0 1 8 3.75Z" />
          <path d="M13.5 3.75V8.5h4.75" />
        </>
      );
    case "note":
      return (
        <>
          <path d="M7.75 4.25h8.5a1.5 1.5 0 0 1 1.5 1.5v12.5a1.5 1.5 0 0 1-1.5 1.5h-8.5a1.5 1.5 0 0 1-1.5-1.5V5.75a1.5 1.5 0 0 1 1.5-1.5Z" />
          <path d="M9.5 9h5M9.5 12h5M9.5 15h3.5" strokeLinecap="round" />
        </>
      );
    case "app":
      return (
        <>
          <rect x="4.25" y="4.25" width="6.5" height="6.5" rx="1.25" />
          <rect x="13.25" y="4.25" width="6.5" height="6.5" rx="1.25" />
          <rect x="4.25" y="13.25" width="6.5" height="6.5" rx="1.25" />
          <rect x="13.25" y="13.25" width="6.5" height="6.5" rx="1.25" />
        </>
      );
    case "folder":
      return (
        <>
          <path d="M3.75 7.75A1.75 1.75 0 0 1 5.5 6h3.17c.46 0 .9.18 1.22.5l1.11 1.11c.33.33.77.51 1.23.51h6.27a1.75 1.75 0 0 1 1.75 1.75v7.63a1.75 1.75 0 0 1-1.75 1.75H5.5a1.75 1.75 0 0 1-1.75-1.75V7.75Z" />
          <path d="M3.75 10.25h16.5" strokeLinecap="round" />
        </>
      );
    case "history":
      return (
        <>
          <path d="M4.75 12a7.25 7.25 0 1 0 2.13-5.12" />
          <path d="M4.75 4.75v4.5h4.5" strokeLinecap="round" strokeLinejoin="round" />
          <path d="M12 8.5V12l2.5 1.5" strokeLinecap="round" strokeLinejoin="round" />
        </>
      );
    case "model":
      return (
        <>
          <path
            d="m12 3.75 1.57 4.18 4.18 1.57-4.18 1.57L12 15.25l-1.57-4.18-4.18-1.57 4.18-1.57L12 3.75Z"
            strokeLinejoin="round"
          />
          <path
            d="m18 14.5.9 2.35 2.35.9-2.35.9L18 21l-.9-2.35-2.35-.9 2.35-.9L18 14.5ZM6 14.75l.66 1.74 1.74.66-1.74.66L6 19.55l-.66-1.74-1.74-.66 1.74-.66L6 14.75Z"
            strokeLinejoin="round"
          />
        </>
      );
    case "settings":
      return (
        <>
          <path d="M6 5.25v13.5M12 5.25v13.5M18 5.25v13.5" strokeLinecap="round" />
          <circle cx="6" cy="9" r="1.75" />
          <circle cx="12" cy="15" r="1.75" />
          <circle cx="18" cy="10" r="1.75" />
        </>
      );
    case "filter":
      return <path d="M4.75 6h14.5l-5.5 6.25v4.75l-3.5 1.75v-6.5L4.75 6Z" strokeLinejoin="round" />;
    case "workspace":
      return (
        <>
          <rect x="3.75" y="5" width="6" height="14" rx="1.5" />
          <rect x="10.5" y="5" width="3" height="14" rx="1.5" />
          <rect x="14.25" y="5" width="6" height="14" rx="1.5" />
        </>
      );
    case "x":
      return <path d="m6.5 6.5 11 11m0-11-11 11" strokeLinecap="round" />;
    case "arrow-right":
      return <path d="M5 12h14m-5-5 5 5-5 5" strokeLinecap="round" strokeLinejoin="round" />;
    default:
      return null;
  }
}

export function UiIcon({ name, className, ...props }: Props) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.7}
      aria-hidden="true"
      className={className}
      {...props}
    >
      {paths(name)}
    </svg>
  );
}
