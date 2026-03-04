import { invokeCommand } from "./client";
import type { ReferenceRow, SetReferenceStateInput } from "../types";

export function listReferenceRows(query: string): Promise<ReferenceRow[]> {
  return invokeCommand("list_reference_rows", { query });
}

export function setReferenceState(input: SetReferenceStateInput): Promise<ReferenceRow> {
  return invokeCommand("set_reference_state", { input });
}
