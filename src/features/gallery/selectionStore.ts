import { defineStore } from "pinia";

type SelectionOptions = {
  toggle: boolean;
  range: boolean;
};

export const useGallerySelectionStore = defineStore("gallery-selection", {
  state: () => ({
    selectedIds: [] as string[],
    activeId: null as string | null,
  }),
  actions: {
    select(id: string, options: SelectionOptions, orderedIds?: string[]) {
      const activeIndex = orderedIds && this.activeId ? orderedIds.indexOf(this.activeId) : -1;
      const nextIndex = orderedIds ? orderedIds.indexOf(id) : -1;

      if (options.range && orderedIds && activeIndex >= 0 && nextIndex >= 0) {
        const start = Math.min(activeIndex, nextIndex);
        const end = Math.max(activeIndex, nextIndex);
        const range = orderedIds.slice(start, end + 1);
        const selected = options.toggle ? new Set(this.selectedIds) : new Set<string>();

        for (const rangeId of range) selected.add(rangeId);
        this.selectedIds = orderedIds.filter((itemId) => selected.has(itemId));
        this.activeId = id;
        return;
      }

      if (options.toggle) {
        const selected = new Set(this.selectedIds);
        if (selected.has(id)) selected.delete(id);
        else selected.add(id);

        this.selectedIds = orderedIds
          ? orderedIds.filter((itemId) => selected.has(itemId))
          : [...selected];
        this.activeId = selected.has(id) ? id : this.selectedIds[this.selectedIds.length - 1] ?? null;
        return;
      }

      this.selectedIds = [id];
      this.activeId = id;
    },
    clear() {
      this.selectedIds = [];
      this.activeId = null;
    },
    restore(selectedIds: string[], activeId: string | null) {
      this.selectedIds = [...selectedIds];
      this.activeId = activeId;
    },
    retain(availableIds: Iterable<string>) {
      const available = new Set(availableIds);
      this.selectedIds = this.selectedIds.filter((id) => available.has(id));
      if (this.activeId && !available.has(this.activeId)) {
        this.activeId = this.selectedIds[this.selectedIds.length - 1] ?? null;
      }
    },
  },
});
