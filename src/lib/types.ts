export type WireError = { code: string; message: string };

export type Result<T> = { ok: true; value: T } | { ok: false; error: WireError };

export type NoteSummary = { id: string; title: string; updated: string; tags: string[] };

export type NoteFrontmatter = {
  id: string;
  title: string;
  tags: string[];
  created: string;
  updated: string;
  links: string[];
  references: string[];
};

export type Note = {
  id: string;
  path: string;
  title: string;
  body: string;
  frontmatter: NoteFrontmatter;
};