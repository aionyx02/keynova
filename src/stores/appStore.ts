import { create } from "zustand";
import type { AppInfo } from "../types/app";

interface AppStore {
  apps: AppInfo[];
  searchResults: AppInfo[];
  query: string;
  isLoading: boolean;
  setApps: (apps: AppInfo[]) => void;
  setSearchResults: (results: AppInfo[]) => void;
  setQuery: (query: string) => void;
  setLoading: (loading: boolean) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  apps: [],
  searchResults: [],
  query: "",
  isLoading: false,
  setApps: (apps) => set({ apps }),
  setSearchResults: (results) => set({ searchResults: results }),
  setQuery: (query) => set({ query }),
  setLoading: (isLoading) => set({ isLoading }),
}));
