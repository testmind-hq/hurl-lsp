import { CurlResult, RunResult } from "./protocol";

export type InspectorTab = "request" | "chain" | "result" | "curl";
export type InspectorSnapshot = { runs: RunResult[]; selectedRun: number; curl?: CurlResult; tab: InspectorTab; revealSecrets: boolean };
type DocumentState = InspectorSnapshot;

const MAX_RESULTS = 10;
const MAX_DOCUMENT_STATES = 10;
const emptyState = (): DocumentState => ({ runs: [], selectedRun: -1, tab: "request", revealSecrets: false });
const documentKey = (uri: string, version: number): string => `${uri}@${version}`;

export class InspectorStore {
  private readonly documents = new Map<string, DocumentState>();
  private readonly runOrder: Array<{ key: string; result: RunResult }> = [];
  private currentKey = "";

  selectDocument(uri: string, version: number): void {
    this.currentKey = documentKey(uri, version);
    const state = this.documents.get(this.currentKey) ?? emptyState();
    this.documents.delete(this.currentKey);
    this.documents.set(this.currentKey, state);
    this.pruneDocuments();
  }
  pushRun(result: RunResult): void {
    this.selectDocument(result.uri, result.documentVersion);
    const state = this.current();
    state.runs.push(result);
    this.runOrder.push({ key: this.currentKey, result });
    while (this.runOrder.length > MAX_RESULTS) {
      const oldest = this.runOrder.shift()!;
      const oldState = this.documents.get(oldest.key);
      if (oldState) {
        const index = oldState.runs.indexOf(oldest.result);
        if (index >= 0) oldState.runs.splice(index, 1);
        oldState.selectedRun = oldState.runs.length - 1;
      }
    }
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
  clearCurl(): void { this.current().curl = undefined; }
  select(tab: InspectorTab): void { this.current().tab = tab; }
  selectRun(index: number): void { const state = this.current(); if (index >= 0 && index < state.runs.length) state.selectedRun = index; }
  toggleSecrets(): void { const state = this.current(); state.revealSecrets = !state.revealSecrets; }
  snapshot(): InspectorSnapshot { const state = this.current(); return { ...state, runs: [...state.runs] }; }

  private current(): DocumentState {
    if (!this.documents.has(this.currentKey)) this.documents.set(this.currentKey, emptyState());
    return this.documents.get(this.currentKey)!;
  }

  private pruneDocuments(): void {
    while (this.documents.size > MAX_DOCUMENT_STATES) {
      const oldestKey = this.documents.keys().next().value as string | undefined;
      if (oldestKey === undefined) return;
      this.documents.delete(oldestKey);
      for (let index = this.runOrder.length - 1; index >= 0; index -= 1) {
        if (this.runOrder[index].key === oldestKey) this.runOrder.splice(index, 1);
      }
    }
  }
}
