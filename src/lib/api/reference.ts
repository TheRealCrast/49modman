import { invokeCommand } from "./client";
import type { ListReferenceRowsInput, ListReferenceRowsResult, ReferenceRow, SetReferenceStateInput } from "../types";

export function listReferenceRows(input: ListReferenceRowsInput): Promise<ListReferenceRowsResult> {
  return invokeCommand("list_reference_rows", { input });
}

export function setReferenceState(input: SetReferenceStateInput): Promise<ReferenceRow> {
  return invokeCommand("set_reference_state", { input });
}
