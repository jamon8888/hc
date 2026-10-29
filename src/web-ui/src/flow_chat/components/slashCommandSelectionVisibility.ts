const SELECTED_COMMAND_SELECTOR =
  '[data-alex-part="commandItem"][data-alex-state="selected"]';

export function scrollSelectedSlashCommandIntoView(
  picker: HTMLElement | null,
): void {
  const selectedItem = picker?.querySelector<HTMLElement>(SELECTED_COMMAND_SELECTOR);
  if (typeof selectedItem?.scrollIntoView !== 'function') {
    return;
  }

  selectedItem.scrollIntoView({
    behavior: 'auto',
    block: 'nearest',
    inline: 'nearest',
  });
}
