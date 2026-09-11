import {
  isEditableTarget,
  normalizeKeyCombination,
  normalizeShortcut,
  shortcutDefinitions,
  type ShortcutDefinition,
  type ShortcutScope,
} from "./shortcutRegistry.ts";

export type ShortcutHandler = (event: KeyboardEvent, definition: ShortcutDefinition) => void | boolean;
export type ShortcutHandlers = Record<string, ShortcutHandler>;

export class ShortcutDispatcher {
  private readonly registrations = new Map<ShortcutScope, Map<string, ShortcutHandler>>();
  private activeScope: ShortcutScope | null = null;
  private listening = false;

  register(scope: ShortcutScope, handlers: ShortcutHandlers) {
    const registration = new Map(Object.entries(handlers));
    this.registrations.set(scope, registration);
    if (scope !== "global") this.activeScope = scope;
    else if (this.activeScope === null) this.activeScope = scope;
    this.startListening();
    return () => {
      if (this.registrations.get(scope) === registration) this.registrations.delete(scope);
      if (this.activeScope === scope) {
        const remaining = [...this.registrations.keys()];
        this.activeScope = remaining[remaining.length - 1] ?? null;
      }
      if (this.registrations.size === 0) this.stopListening();
    };
  }

  dispatch(event: KeyboardEvent) {
    if (event.defaultPrevented) return false;
    const normalized = normalizeShortcut(event);
    const activeScope = this.activeScope;
    const activeScopes: ShortcutScope[] = activeScope
      ? ["global", ...(activeScope === "global" ? [] : [activeScope])]
      : [];

    for (let index = activeScopes.length - 1; index >= 0; index -= 1) {
      const scope = activeScopes[index];
      const definition = shortcutDefinitions.find((item) =>
        item.scope === scope
        && item.keys.some((key) => normalizeKeyCombination(key) === normalized)
        && (item.allowRepeat !== false || !event.repeat),
      );
      const handler = definition ? this.registrations.get(scope)?.get(definition.id) : undefined;
      if (!definition || !handler) continue;
      if (isEditableTarget(event.target) && !definition.allowInInput) continue;

      const handled = handler(event, definition);
      if (handled === false) return false;
      event.preventDefault();
      return true;
    }

    return false;
  }

  private readonly handleKeydown = (event: KeyboardEvent) => {
    this.dispatch(event);
  };

  private startListening() {
    if (this.listening || typeof window === "undefined") return;
    window.addEventListener("keydown", this.handleKeydown);
    this.listening = true;
  }

  private stopListening() {
    if (!this.listening || typeof window === "undefined") return;
    window.removeEventListener("keydown", this.handleKeydown);
    this.listening = false;
  }
}

export const shortcutDispatcher = new ShortcutDispatcher();
