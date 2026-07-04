export type WireError = { code: string; message: string };

export type Result<T> = { ok: true; value: T } | { ok: false; error: WireError };
