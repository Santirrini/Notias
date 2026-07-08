/**
 * Pure helpers around a ProseMirror EditorView.
 *
 * Every UI surface that needs to read or write the note's body — toolbar,
 * slash menu, ghost-text, status bar — talks to the view through this module
 * so the same serialization / parsing / cursor math is reused everywhere.
 *
 * The serializer/parser are bound to the editor's ctx, so most operations
 * take the `Editor` (for ctx access) plus the `EditorView` (for the
 * dispatchable transaction). Pass `null` for either to no-op.
 */

import type { Editor } from "@milkdown/core";
import { parserCtx, serializerCtx } from "@milkdown/core";
import type { EditorView } from "prosemirror-view";

/** Read the current markdown out of the editor. Returns "" if either is gone. */
export function serialize(
  editor: Editor | null,
  view: EditorView | null,
): string {
  if (!editor || !view) return "";
  return editor.action((ctx) => {
    return ctx.get(serializerCtx)(view.state.doc);
  });
}

/** Replace the entire document with new markdown. */
export function setMarkdown(
  editor: Editor | null,
  view: EditorView | null,
  md: string,
): void {
  if (!editor || !view) return;
  editor.action((ctx) => {
    const parser = ctx.get(parserCtx);
    const doc = parser(md);
    const { state, dispatch } = view;
    const tr = state.tr.replaceWith(0, state.doc.content.size, doc as never);
    dispatch(tr);
  });
}

/** Insert plain text at the current selection, replacing it if non-empty. */
export function insertTextAtCursor(
  view: EditorView | null | undefined,
  text: string,
): void {
  if (!view) return;
  const { state, dispatch } = view;
  const { from, to } = state.selection;
  const tr = state.tr.insertText(text, from, to);
  dispatch(tr);
}

/** Pixel rect of the current cursor (1 = below the caret). */
export function coordsAtCursor(view: EditorView | null | undefined) {
  if (!view) return null;
  const { state } = view;
  const head = state.selection.head ?? state.selection.from;
  return view.coordsAtPos(head, 1);
}

/** Focus the editor. */
export function focusView(view: EditorView | null | undefined) {
  view?.focus();
}

/** True when the view currently has focus. */
export function viewHasFocus(view: EditorView | null | undefined): boolean {
  if (!view) return false;
  return view.hasFocus();
}
