/**
 * Editor command registry — the toolbar and slash menu both pull from this
 * single source so what you can do via the keyboard matches what you can do
 * via the mouse and what shows up in the slash menu.
 *
 * Each command knows:
 *   - how to render itself (icon, label, group)
 *   - how to execute against an editor (`run`)
 *   - whether it's currently active in the selection (for "pressed" styling)
 *
 * Commands are intentionally data-driven so we can serialize them later
 * (e.g. for user-defined commands or AI-suggested actions).
 */

import type { Editor } from "@milkdown/core";
import type { EditorView } from "prosemirror-view";
import {
  Bold,
  Italic,
  Strikethrough,
  Code,
  Heading1,
  Heading2,
  Heading3,
  List,
  ListOrdered,
  Quote,
  Code2,
  Minus,
  Sparkles,
  Wand2,
  ListChecks,
  Tags,
  type Icon as IconType,
} from "@lucide/svelte";
import {
  insertTextAtCursor,
  serialize,
  setMarkdown,
} from "./serialize";
import { safeAiComplete, safeAiSummarize } from "$lib/stores/backend.svelte";
import { toast } from "svelte-sonner";

export type CommandGroup = "inline" | "block" | "ai";

export interface EditorCommand {
  id: string;
  label: string;
  description?: string;
  icon: typeof IconType;
  shortcut?: string;
  group: CommandGroup;
  run: (ctx: CommandContext) => void | Promise<void>;
}

export interface CommandContext {
  editor: Editor | null;
  view: EditorView | null;
  body: string;
  title: string;
}

// ---------------------------------------------------------------------------
// Inline marks — implemented as find/replace on the current selection range
// against the serialized markdown. Robust across the commonmark preset
// (no toggleMark API to maintain), and good enough for our use case.
// ---------------------------------------------------------------------------

function wrapSelection(view: EditorView, before: string, after: string) {
  const { state, dispatch } = view;
  const { from, to, empty } = state.selection;
  if (empty) {
    // No selection — insert a pair with the cursor between them, all in one tx.
    const tr = state.tr
      .insertText(`${before}${after}`, from, to)
      .setSelection(
        // TextSelection.create is the public API on ProseMirror's TextSelection
        // class; we construct via state.selection.constructor which is the
        // same object instance at runtime.
        (state.selection.constructor as unknown as {
          create: (doc: unknown, pos: number) => unknown;
        }).create(state.doc, from + before.length) as never,
      );
    dispatch(tr);
    return;
  }
  const selected = state.doc.textBetween(from, to, "\n", "\n");
  const tr = state.tr.insertText(`${before}${selected}${after}`, from, to);
  dispatch(tr);
}

function toggleLinePrefix(
  editor: Editor,
  view: EditorView,
  prefixes: string[],
  fallback: string,
) {
  const md = serialize(editor, view);
  const { state, dispatch } = view;
  // Determine the line range that contains the selection.
  const fromPos = state.selection.$from;
  const start = fromPos.start();
  const end = fromPos.end();
  const lines = state.doc.textBetween(start, end, "\n", "\n").split("\n");
  const transformed = lines.map((line: string) => {
    const stripped = prefixes.find((p) => line.startsWith(p));
    if (stripped) return line.slice(stripped.length).replace(/^\s+/, "");
    return `${fallback}${line}`;
  });
  const replacement = transformed.join("\n");
  const tr = state.tr.insertText(replacement, start, end);
  dispatch(tr);
  void md;
}

const inlineCommands: EditorCommand[] = [
  {
    id: "bold",
    label: "Bold",
    description: "Bold the selected text",
    icon: Bold,
    shortcut: "Ctrl+B",
    group: "inline",
    run: ({ view }) => {
      if (view) wrapSelection(view, "**", "**");
    },
  },
  {
    id: "italic",
    label: "Italic",
    description: "Italicize the selected text",
    icon: Italic,
    shortcut: "Ctrl+I",
    group: "inline",
    run: ({ view }) => {
      if (view) wrapSelection(view, "*", "*");
    },
  },
  {
    id: "strike",
    label: "Strikethrough",
    description: "Strikethrough the selected text",
    icon: Strikethrough,
    group: "inline",
    run: ({ view }) => {
      if (view) wrapSelection(view, "~~", "~~");
    },
  },
  {
    id: "code",
    label: "Inline code",
    description: "Format selection as inline code",
    icon: Code,
    shortcut: "Ctrl+E",
    group: "inline",
    run: ({ view }) => {
      if (view) wrapSelection(view, "`", "`");
    },
  },
];

const blockCommands: EditorCommand[] = [
  {
    id: "h1",
    label: "Heading 1",
    description: "Large section heading",
    icon: Heading1,
    group: "block",
    run: ({ editor, view }) => {
      if (editor && view) toggleLinePrefix(editor, view, ["# "], "# ");
    },
  },
  {
    id: "h2",
    label: "Heading 2",
    description: "Medium section heading",
    icon: Heading2,
    group: "block",
    run: ({ editor, view }) => {
      if (editor && view) toggleLinePrefix(editor, view, ["## "], "## ");
    },
  },
  {
    id: "h3",
    label: "Heading 3",
    description: "Small section heading",
    icon: Heading3,
    group: "block",
    run: ({ editor, view }) => {
      if (editor && view) toggleLinePrefix(editor, view, ["### "], "### ");
    },
  },
  {
    id: "ul",
    label: "Bulleted list",
    description: "Unordered list",
    icon: List,
    group: "block",
    run: ({ editor, view }) => {
      if (editor && view) toggleLinePrefix(editor, view, ["- ", "* "], "- ");
    },
  },
  {
    id: "ol",
    label: "Numbered list",
    description: "Ordered list",
    icon: ListOrdered,
    group: "block",
    run: ({ editor, view }) => {
      if (editor && view) toggleLinePrefix(editor, view, ["1. "], "1. ");
    },
  },
  {
    id: "quote",
    label: "Quote",
    description: "Block quote",
    icon: Quote,
    group: "block",
    run: ({ editor, view }) => {
      if (editor && view) toggleLinePrefix(editor, view, ["> "], "> ");
    },
  },
  {
    id: "codeblock",
    label: "Code block",
    description: "Fenced code block",
    icon: Code2,
    group: "block",
    run: ({ editor, view }) => {
      if (editor && view) toggleLinePrefix(editor, view, ["```\n", "```\n"], "```\n");
    },
  },
  {
    id: "divider",
    label: "Divider",
    description: "Horizontal rule",
    icon: Minus,
    group: "block",
    run: ({ editor, view }) => {
      if (!editor || !view) return;
      const md = serialize(editor, view);
      setMarkdown(editor, view, `${md.trimEnd()}\n\n---\n\n`);
    },
  },
];

// ---------------------------------------------------------------------------
// AI commands — toast on failure, insert result at cursor on success.
// ---------------------------------------------------------------------------

async function aiContinuation(ctx: CommandContext) {
  const completion = await safeAiComplete(`${ctx.body}\n`);
  if (completion == null) {
    toast.error("AI unavailable", {
      description: "Check the offline banner or provider settings.",
    });
    return;
  }
  const cleaned = stripContinuation(completion);
  if (view(ctx)) insertTextAtCursor(ctx.view, "\n\n" + cleaned + "\n\n");
}

async function aiSummarize(ctx: CommandContext) {
  if (!ctx.body.trim()) {
    toast.info("Nothing to summarize yet");
    return;
  }
  const summary = await safeAiSummarize(ctx.body, "concise");
  if (summary == null) {
    toast.error("AI unavailable");
    return;
  }
  if (ctx.view) {
    setMarkdown(ctx.editor, ctx.view, `${ctx.body.trimEnd()}\n\n---\n\n**Summary**\n\n${summary}\n`);
  }
}

async function aiTags(ctx: CommandContext) {
  if (!ctx.body.trim()) {
    toast.info("Add some text first");
    return;
  }
  const completion = await safeAiComplete(
    `Suggest 3 to 5 short topic tags for the following note, comma-separated, no hashtags, no explanation:\n\n${ctx.body}\n\nTags:`,
  );
  if (completion == null) {
    toast.error("AI unavailable");
    return;
  }
  const tags = completion
    .split(/[,\n]/)
    .map((t) => t.trim().replace(/^#/, "").toLowerCase())
    .filter(Boolean)
    .slice(0, 5);
  if (tags.length === 0) {
    toast.info("No tags suggested");
    return;
  }
  toast.success("Suggested tags", {
    description: tags.map((t) => `#${t}`).join("  "),
  });
}

const aiCommands: EditorCommand[] = [
  {
    id: "ai-suggest",
    label: "Continue writing",
    description: "AI continues from the cursor",
    icon: Sparkles,
    shortcut: "Ctrl+J",
    group: "ai",
    run: aiContinuation,
  },
  {
    id: "ai-summarize",
    label: "Summarize note",
    description: "Append a concise summary",
    icon: Wand2,
    group: "ai",
    run: aiSummarize,
  },
  {
    id: "ai-tags",
    label: "Suggest tags",
    description: "AI suggests topic tags",
    icon: Tags,
    group: "ai",
    run: aiTags,
  },
  {
    id: "ai-flashcards",
    label: "Generate flashcards",
    description: "Build SRS cards from this note",
    icon: ListChecks,
    group: "ai",
    run: () => {
      toast.info("Use the Study tab", {
        description: "Open /study to generate flashcards from this note.",
      });
    },
  },
];

export const allCommands: EditorCommand[] = [
  ...inlineCommands,
  ...blockCommands,
  ...aiCommands,
];

export function findCommand(id: string): EditorCommand | undefined {
  return allCommands.find((c) => c.id === id);
}

export function runCommandById(
  id: string,
  ctx: CommandContext,
): void | Promise<void> {
  const cmd = findCommand(id);
  if (!cmd) return;
  return cmd.run(ctx);
}

function view(ctx: CommandContext): EditorView | null {
  return ctx.view;
}

function stripContinuation(s: string): string {
  const cut = s.indexOf("\n\n");
  const out = cut >= 0 ? s.slice(0, cut) : s;
  return out.trim();
}
