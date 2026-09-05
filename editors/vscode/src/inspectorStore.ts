import { CurlResult, RunResult } from "./protocol";

export type InspectorTab = "request" | "chain" | "result" | "curl";
export type InspectorSnapshot = { runs: RunResult[]; selectedRun: number; curl?: CurlResult; tab: InspectorTab; revealSecrets: boolean };

export class InspectorStore {
  private runs: RunResult[] = [];
  private selectedRun = -1;
  private curl?: CurlResult;
  private tab: InspectorTab = "request";
  private revealSecrets = false;

  pushRun(result: RunResult): void {
    this.runs.push(result);
    if (this.runs.length > 10) this.runs.splice(0, this.runs.length - 10);
    this.selectedRun = this.runs.length - 1;
    this.tab = "result";
    this.revealSecrets = false;
  }
  setCurl(result: CurlResult): void { this.curl = result; this.tab = "curl"; this.revealSecrets = false; }
  select(tab: InspectorTab): void { this.tab = tab; }
  selectRun(index: number): void { if (index >= 0 && index < this.runs.length) this.selectedRun = index; }
  toggleSecrets(): void { this.revealSecrets = !this.revealSecrets; }
  snapshot(): InspectorSnapshot { return { runs: [...this.runs], selectedRun: this.selectedRun, curl: this.curl, tab: this.tab, revealSecrets: this.revealSecrets }; }
}
