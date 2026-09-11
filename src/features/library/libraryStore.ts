import { invoke } from "@tauri-apps/api/core";
import { defineStore } from "pinia";
import { shallowReactive } from "vue";
import type {
  FolderSelection,
  ImageAsset,
  Library,
  LibraryCommandError,
  ScanResult,
  ScannerErrorCode,
  ScannerState,
} from "./types";

const isTauriEnvironment = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function sortLibraries(libraries: Library[]) {
  return [...libraries].sort((left, right) => right.updatedAt - left.updatedAt);
}

function errorCode(error: unknown): LibraryCommandError["code"] {
  const value = typeof error === "string" ? parseError(error) : error;

  if (value && typeof value === "object" && "code" in value) {
    const code = value.code;
    if (
      code === "dialog_failed" ||
      code === "invalid_path" ||
      code === "duplicate_path" ||
      code === "persistence_failed"
    ) {
      return code;
    }
  }

  return "persistence_failed";
}

function parseError(error: string) {
  try {
    return JSON.parse(error) as unknown;
  } catch {
    return error;
  }
}

function scannerErrorCode(error: unknown): ScannerErrorCode {
  const value = typeof error === "string" ? parseError(error) : error;

  if (value && typeof value === "object" && "code" in value) {
    const code = value.code;
    if (
      code === "library_not_found" ||
      code === "library_unavailable" ||
      code === "permission_denied" ||
      code === "scan_failed" ||
      code === "persistence_failed" ||
      code === "scan_in_progress"
    ) {
      return code;
    }
  }

  return "scan_failed";
}

function scannerState(status: ScannerState["status"], error: ScannerErrorCode | null = null): ScannerState {
  return {
    status,
    progress: status === "completed" ? 1 : null,
    total: null,
    current: null,
    error,
  };
}

export const useLibraryStore = defineStore("library", {
  state: () => ({
    libraries: [] as Library[],
    error: null as LibraryCommandError["code"] | null,
    isAdding: false,
    isLoading: false,
    removingId: null as string | null,
    // Asset lists are replaced as complete scan results; individual assets never mutate in Vue.
    // Keeping the container shallow prevents a large library from becoming a deep reactive tree.
    assetsByLibrary: shallowReactive({}) as Record<string, ImageAsset[]>,
    scannerStates: {} as Record<string, ScannerState>,
  }),
  actions: {
    async loadLibraries() {
      if (!isTauriEnvironment) return;

      this.isLoading = true;
      this.error = null;

      try {
        this.libraries = sortLibraries(await invoke<Library[]>("list_libraries"));
      } catch (error) {
        this.error = errorCode(error);
      } finally {
        this.isLoading = false;
      }
    },
    async addLibrary() {
      if (!isTauriEnvironment) return;

      this.isAdding = true;
      this.error = null;

      try {
        const folder = await invoke<FolderSelection | null>("select_folder");
        if (!folder) return;

        const library = await invoke<Library>("create_library", { path: folder.path });
        this.libraries = sortLibraries([
          ...this.libraries.filter((item) => item.id !== library.id),
          library,
        ]);
      } catch (error) {
        this.error = errorCode(error);
      } finally {
        this.isAdding = false;
      }
    },
    async removeLibrary(id: string) {
      if (!isTauriEnvironment) return;

      this.removingId = id;
      this.error = null;

      try {
        await invoke("remove_library", { id });
        this.libraries = this.libraries.filter((library) => library.id !== id);
        delete this.assetsByLibrary[id];
        delete this.scannerStates[id];
      } catch (error) {
        this.error = errorCode(error);
      } finally {
        this.removingId = null;
      }
    },
    async ensureLibraryAssets(libraryId: string) {
      if (!isTauriEnvironment) return;

      if (!this.libraries.some((library) => library.id === libraryId)) {
        await this.loadLibraries();
      }
      if (!this.libraries.some((library) => library.id === libraryId)) return;
      if (!(libraryId in this.assetsByLibrary)) {
        await this.scanLibrary(libraryId);
      }
    },
    async refreshLibraryAssets(libraryId: string) {
      if (!isTauriEnvironment) return;

      if (!this.libraries.some((library) => library.id === libraryId)) {
        await this.loadLibraries();
      }
      if (this.libraries.some((library) => library.id === libraryId)) {
        await this.scanLibrary(libraryId);
      }
    },
    async scanLibrary(libraryId: string) {
      if (!isTauriEnvironment || this.scannerStates[libraryId]?.status === "scanning") return;

      this.scannerStates[libraryId] = scannerState("scanning");

      try {
        const result = await invoke<ScanResult>("scan_library", { libraryId });
        const count = result.assets.length;

        this.assetsByLibrary[libraryId] = result.assets;
        this.scannerStates[libraryId] = {
          ...scannerState("completed"),
          total: count,
          current: count,
        };
        this.libraries = sortLibraries([
          ...this.libraries.filter((library) => library.id !== result.library.id),
          result.library,
        ]);
      } catch (error) {
        delete this.assetsByLibrary[libraryId];
        this.scannerStates[libraryId] = scannerState("error", scannerErrorCode(error));
      }
    },
  },
});
