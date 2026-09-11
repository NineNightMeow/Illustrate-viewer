export function isPointerInteractiveTarget(target: EventTarget | null) {
  return typeof Element !== "undefined" && target instanceof Element
    && Boolean(target.closest(
      "button, a[href], input, textarea, select, [contenteditable], "
      + "[role='button'], [role='link'], [role='menuitem'], [role='textbox'], [type='range']",
    ));
}
