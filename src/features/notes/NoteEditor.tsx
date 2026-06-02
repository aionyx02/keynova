import { useCallback, useEffect, useRef, useState } from "react";
import { useNotes } from "../../hooks/useNotes";
import { useI18n } from "../../i18n/useI18n";
import type { PanelProps } from "../../types/panel";

interface NoteDraftPayload {
  draft_name?: string;
  draft_content?: string;
}

function parseInitialArgs(initialArgs?: string): {
  selectedNote?: string;
  draft?: NoteDraftPayload;
} {
  const value = initialArgs?.trim();
  if (!value) return {};
  try {
    const parsed = JSON.parse(value) as NoteDraftPayload;
    if (parsed && (parsed.draft_name || parsed.draft_content)) {
      return { draft: parsed };
    }
  } catch {
    // Plain string note name.
  }
  return { selectedNote: value };
}

export function NoteEditor({ onClose, initialArgs }: PanelProps) {
  const t = useI18n();
  const { notes, getNote, saveNote, createNote, deleteNote } = useNotes();
  const parsedInitial = parseInitialArgs(initialArgs);
  const [selectedNote, setSelectedNote] = useState<string | null>(
    parsedInitial.selectedNote ?? null,
  );
  const [content, setContent] = useState(parsedInitial.draft?.draft_content ?? "");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved" | "error">("idle");
  const [newNoteName, setNewNoteName] = useState(parsedInitial.draft?.draft_name ?? "");
  const [showCreate, setShowCreate] = useState(Boolean(parsedInitial.draft));
  const [confirmDelete, setConfirmDelete] = useState(false);
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!selectedNote) return;
    getNote(selectedNote)
      .then((nextContent) => {
        setContent(nextContent);
        setSaveStatus("idle");
      })
      .catch(() => {
        setContent("");
        setSaveStatus("error");
      });
  }, [selectedNote, getNote]);

  useEffect(() => {
    if (selectedNote) {
      editorRef.current?.focus();
    }
  }, [selectedNote]);

  const triggerSave = useCallback(() => {
    if (!selectedNote) return;
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(async () => {
      try {
        await saveNote(selectedNote, content);
        setSaveStatus("saved");
        setTimeout(() => setSaveStatus("idle"), 1500);
      } catch {
        setSaveStatus("error");
      }
    }, 800);
  }, [selectedNote, content, saveNote]);

  function handleEditorKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "s" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      if (!selectedNote) return;
      if (timerRef.current) clearTimeout(timerRef.current);
      void saveNote(selectedNote, content)
        .then(() => {
          setSaveStatus("saved");
          setTimeout(() => setSaveStatus("idle"), 1500);
        })
        .catch(() => setSaveStatus("error"));
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      if (selectedNote !== null) {
        setSelectedNote(null);
        setContent("");
      } else {
        onClose();
      }
    }
  }

  async function handleCreate() {
    const name = newNoteName.trim();
    if (!name) return;
    const draftContent = content;
    await createNote(name);
    if (draftContent.trim()) {
      await saveNote(name, draftContent);
    }
    setNewNoteName("");
    setShowCreate(false);
    setSelectedNote(name);
    setContent(draftContent);
  }

  async function handleDelete() {
    if (!selectedNote) return;
    await deleteNote(selectedNote);
    setSelectedNote(null);
    setContent("");
    setConfirmDelete(false);
  }

  return (
    <div className="kn-panel-shell flex rounded-t-none border-t-0" style={{ height: 380 }}>
      <div className="flex w-40 flex-col border-r border-[color:var(--kn-border)] bg-white/[0.015]">
        <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] px-2 py-2">
          <span className="text-[10px] font-semibold uppercase text-[color:var(--kn-text-faint)]">
            {t.note.title}
          </span>
          <button
            onClick={() => setShowCreate(true)}
            className="kn-button h-7 px-2 py-1 text-xs font-bold"
            title={t.note.new}
          >
            +
          </button>
        </div>
        <div className="kn-scroll flex-1 overflow-y-auto p-1">
          {notes.map((note) => (
            <button
              key={note.name}
              onClick={() => {
                setSelectedNote(note.name);
                setShowCreate(false);
              }}
              className={`kn-sidebar-item w-full truncate px-2 py-1.5 text-left text-xs ${
                selectedNote === note.name
                  ? "text-[color:var(--kn-text)]"
                  : "text-[color:var(--kn-text-muted)]"
              }`}
              data-active={selectedNote === note.name ? "true" : "false"}
            >
              {note.name}
            </button>
          ))}
          {notes.length === 0 && (
            <p className="mt-4 text-center text-[10px] text-[color:var(--kn-text-faint)]">
              No notes yet.
            </p>
          )}
        </div>
        {showCreate && (
          <div className="flex gap-1 border-t border-[color:var(--kn-border)] p-2">
            <input
              autoFocus
              value={newNoteName}
              onChange={(e) => setNewNoteName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void handleCreate();
                if (e.key === "Escape") setShowCreate(false);
              }}
              placeholder={t.note.namePlaceholder}
              className="kn-field flex-1 px-1 py-0.5 text-[10px]"
            />
            <button
              onClick={() => void handleCreate()}
              className="text-[10px] text-[color:var(--kn-accent)] transition-colors hover:text-[color:var(--kn-accent-strong)]"
            >
              Create
            </button>
          </div>
        )}
      </div>

      <div className="flex-1 flex flex-col">
        {selectedNote ? (
          <>
            <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] bg-white/[0.015] px-3 py-1.5">
              <span className="text-xs font-medium text-[color:var(--kn-text-soft)]">
                {selectedNote}.md
              </span>
              <div className="flex items-center gap-3">
                {saveStatus === "saved" && (
                  <span className="text-[10px] text-[color:var(--kn-success)]">{t.note.saved}</span>
                )}
                {saveStatus === "error" && (
                  <span className="text-[10px] text-[color:var(--kn-danger)]">Save failed</span>
                )}
                {confirmDelete ? (
                  <span className="flex gap-2 text-[10px] text-[color:var(--kn-danger)]">
                    Delete?
                    <button
                      onClick={() => void handleDelete()}
                      className="transition-colors hover:text-[color:var(--kn-danger-strong)]"
                    >
                      Yes
                    </button>
                    <button
                      onClick={() => setConfirmDelete(false)}
                      className="text-[color:var(--kn-text-muted)] transition-colors hover:text-[color:var(--kn-text-soft)]"
                    >
                      No
                    </button>
                  </span>
                ) : (
                  <button
                    onClick={() => setConfirmDelete(true)}
                    className="text-[10px] text-[color:var(--kn-text-faint)] transition-colors hover:text-[color:var(--kn-danger)]"
                  >
                    {t.note.delete}
                  </button>
                )}
              </div>
            </div>
            <textarea
              ref={editorRef}
              value={content}
              onChange={(e) => {
                setContent(e.target.value);
                triggerSave();
              }}
              onKeyDown={handleEditorKeyDown}
              placeholder={t.note.placeholder}
              className="kn-scroll flex-1 resize-none bg-transparent px-3 py-2 font-mono text-sm text-[color:var(--kn-text)] outline-none placeholder:text-[color:var(--kn-text-faint)]"
            />
            <div className="px-3 py-1 text-[10px] text-[color:var(--kn-text-faint)]">
              Ctrl+S to save, Esc to go back.
            </div>
          </>
        ) : (
          <div className="flex flex-1 items-center justify-center text-sm text-[color:var(--kn-text-faint)]">
            {showCreate ? "Create a note to continue." : "Select a note or create a draft."}
          </div>
        )}
      </div>
    </div>
  );
}
