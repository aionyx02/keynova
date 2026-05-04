import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface NoteMeta {
  name: string;
  filename: string;
  size_bytes: number;
  modified: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function useNotes() {
  const [notes, setNotes] = useState<NoteMeta[]>([]);

  const refresh = useCallback(async () => {
    if (!window.__TAURI_INTERNALS__) return;
    try {
      const list = await ipcDispatch<NoteMeta[]>("note.list");
      setNotes(list);
    } catch {
      setNotes([]);
    }
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    ipcDispatch<NoteMeta[]>("note.list")
      .then(setNotes)
      .catch(() => setNotes([]));
  }, []);

  const getNote = useCallback(async (name: string): Promise<string> => {
    const result = await ipcDispatch<{ name: string; content: string }>("note.get", { name });
    return result.content;
  }, []);

  const saveNote = useCallback(async (name: string, content: string) => {
    await ipcDispatch("note.save", { name, content });
    await refresh();
  }, [refresh]);

  const createNote = useCallback(async (name: string) => {
    await ipcDispatch("note.create", { name });
    await refresh();
  }, [refresh]);

  const deleteNote = useCallback(async (name: string) => {
    await ipcDispatch("note.delete", { name });
    await refresh();
  }, [refresh]);

  const renameNote = useCallback(async (oldName: string, newName: string) => {
    await ipcDispatch("note.rename", { old_name: oldName, new_name: newName });
    await refresh();
  }, [refresh]);

  return { notes, refresh, getNote, saveNote, createNote, deleteNote, renameNote };
}