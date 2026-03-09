import { createSignal } from "solid-js";
import type { AnalysisSummary } from "../lib/types";
import * as ipc from "../lib/ipc";

const [analysis, setAnalysis] = createSignal<AnalysisSummary | null>(null);
const [analyzing, setAnalyzing] = createSignal(false);
const [analysisError, setAnalysisError] = createSignal<string | null>(null);

export { analysis, analyzing, analysisError };

export async function runAnalysis(accountId: string) {
  setAnalyzing(true);
  setAnalysisError(null);
  try {
    const result = await ipc.runAnalysis(accountId);
    setAnalysis(result);
    return result;
  } catch (e) {
    setAnalysisError(String(e));
    return null;
  } finally {
    setAnalyzing(false);
  }
}
