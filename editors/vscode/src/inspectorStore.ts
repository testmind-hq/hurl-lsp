import { CurlResult, RunResult } from "./protocol";

export type InspectorTab = "request" | "chain" | "result" | "curl";
export type InspectorSnapshot = { runs: RunResult[]; selectedRun: number; curl?: CurlResult; tab: InspectorTab; revealSecrets: boolean };
type DocumentState = InspectorSnapshot;

const emptyState = (): DocumentState => ({ runs: [], selectedRun: -1, tab: "request", revealSecrets: false });
const documentKey = (uri: string, version: number): string => `${uri}@${version}`;

export class InspectorStore {
  private readonly documents = new Map<string, DocumentState>();
  private currentKey = "";

  selectDocument(uri: string, version: number): void {
    this.currentKey = documentKey(uri, version);
    if (!this.documents.has(this.currentKey)) this.documents.set(this.currentKey, emptyState());
  }
  pushRun(result: RunResult): void {
    this.selectDocument(result.uri, result.documentVersion);
    const state = this.current();
    state.runs.push(result);
    if (state.runs.length > 10) state.runs.splice(0, state.runs.length - 10);
    state.selectedRun = state.runs.length - 1;
    state.tab = "result";
    state.revealSecrets = false;
  }
  setCurl(result: CurlResult): void {
    this.selectDocument(result.uri, result.documentVersion);
    const state = this.current();
    state.curl = result;
    state.tab = "curl";
    state.revealSecrets = false;
  }
  select(tab: InspectorTab): void { this.current().tab = tab; }
  selectRun(index: number): void { const state = this.current(); if (index >= 0 && index < state.runs.length) state.selectedRun = index; }
  toggleSecrets(): void { const state = this.current(); state.revealSecrets = !state.revealSecrets; }
  snapshot(): InspectorSnapshot { const state = this.current(); return { ...state, runs: [...state.runs] }; }

  private current(): DocumentState {
    if (!this.documents.has(this.currentKey)) this.documents.set(this.currentKey, emptyState());
    return this.documents.get(this.currentKey)!;
  }
}
