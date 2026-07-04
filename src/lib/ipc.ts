import { invoke } from "@tauri-apps/api/core";
import type { WireError } from "./types";

export async function ping(): Promise<string> {
  try {
    return await invoke<string>("ping");
  } catch (e) {
    throw e as WireError;
  }
}
