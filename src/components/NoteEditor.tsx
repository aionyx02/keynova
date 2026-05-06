import { useCallback, useEffect, useRef, useState } from "react";
import { useNotes } from "../hooks/useNotes";
import { useI18n } from "../i18n/useI18n";
import type { PanelProps } from "../types/panel";

export function NoteEditor({ onClose }: PanelProps) {
  const t = useI18n();
  const { notes, getNote, saveNote, createNote, deleteNote } = useNotes();
  const [selectedNote, setSelectedNote] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved" | "error">("idle");
  const [newNoteName, setNewNoteName] = useState("");
  const [showCreate, setShowCreate] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const editorRef = useRef<HTMLTextAreaElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (selectedNote) {
      getNote(selectedNote)
        .then((c) => { setContent(c); setSaveStatus("idle"); })
        .catch(() => { setContent(""); setSaveStatus("error"); });
    }
  }, [selectedNote, getNote]);

  useEffect(() => {
    if (selectedNote) editorRef.current?.focus();
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
      if (timerRef.current) clearTimeout(timerRef.current);
      void saveNote(selectedNote!, content).then(() => {
        setSaveStatus("saved");
        setTimeout(() => setSaveStatus("idle"), 1500);
      });
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      // First Escape: deselect note; second Escape (no note selected): close panel
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
    await createNote(name);
    setNewNoteName("");
    setShowCreate(false);
    setSelectedNote(name);
  }

  async function handleDelete() {
    if (!selectedNote) return;
    await deleteNote(selectedNote);
    setSelectedNote(null);
    setContent("");
    setConfirmDelete(false);
  }

  return (
    <div className="min-h-h-[350] bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex" style={{ height: 380 }}>
      {/* Sidebar: note list */}
      <div className="w-36 border-r border-gray-700/50 flex flex-col">
        <div className="flex items-center justify-between px-2 py-2 border-b border-gray-700/30">
          <span className="text-[10px] font-semibold text-gray-500 uppercase">{t.note.title}</span>
          <button
            onClick={() => setShowCreate(true)}
            className="text-blue-400 hover:text-blue-300 text-xs font-bold"
            title={t.note.new}
          >+</button>
        </div>
        <div className="flex-1 overflow-y-auto">
          {notes.map((note) => (
            <button
              key={note.name}
              onClick={() => setSelectedNote(note.name)}
              className={`w-full text-left px-2 py-1.5 text-xs truncate transition-colors ${
                selectedNote === note.name
                  ? "bg-blue-600/40 text-white"
                  : "text-gray-400 hover:bg-white/5"
              }`}
            >
              {note.name}
            </button>
          ))}
          {notes.length === 0 && (
            <p className="text-center text-gray-700 text-[10px] mt-4">+新增筆記</p>
          )}
        </div>
        {showCreate && (
          <div className="p-2 border-t border-gray-700/30 flex gap-1">
            <input
              autoFocus
              value={newNoteName}
              onChange={(e) => setNewNoteName(e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") void handleCreate(); if (e.key === "Escape") setShowCreate(false); }}
              placeholder={t.note.namePlaceholder}
              className="flex-1 bg-gray-800/60 text-gray-200 text-[10px] rounded px-1 py-0.5 outline-none"
            />
            <button onClick={() => void handleCreate()} className="text-blue-400 text-[10px] hover:text-blue-300">✓</button>
          </div>
        )}
      </div>

      {/* Editor area */}
      <div className="flex-1 flex flex-col">
        {selectedNote ? (
          <>
            <div className="flex items-center justify-between px-3 py-1.5 border-b border-gray-700/30">
              <span className="text-xs text-gray-400 font-medium">{selectedNote}.md</span>
              <div className="flex items-center gap-3">
                {saveStatus === "saved" && <span className="text-[10px] text-green-400">✓ {t.note.saved}</span>}
                {saveStatus === "error" && <span className="text-[10px] text-red-400">儲存失敗</span>}
                {confirmDelete ? (
                  <span className="text-[10px] text-red-400 flex gap-2">
                    確定？
                    <button onClick={() => void handleDelete()} className="hover:text-red-300">是</button>
                    <button onClick={() => setConfirmDelete(false)} className="hover:text-gray-300">否</button>
                  </span>
                ) : (
                  <button
                    onClick={() => setConfirmDelete(true)}
                    className="text-[10px] text-gray-600 hover:text-red-400 transition-colors"
                  >{t.note.delete}</button>
                )}
              </div>
            </div>
            <textarea
              ref={editorRef}
              value={content}
              onChange={(e) => { setContent(e.target.value); triggerSave(); }}
              onKeyDown={handleEditorKeyDown}
              placeholder={t.note.placeholder}
              className="flex-1 bg-transparent text-gray-200 text-sm px-3 py-2 outline-none resize-none font-mono placeholder-gray-700"
            />
            <div className="px-3 py-1 text-[10px] text-gray-700">Ctrl+S 儲存 · Esc 返回</div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-gray-700 text-sm">
            ← 選擇或新增筆記
          </div>
        )}
      </div>
    </div>
  );
}