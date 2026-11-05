import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';

/** Debe coincidir con `INTENTIONAL_FAILURE_CODE` en `src-tauri/src/commands.rs`. */
const INTENTIONAL_FAILURE_PREFIX = 'INTENTIONAL_FAILURE: ';

export type BridgeResult =
  | { kind: 'success'; message: string }
  | { kind: 'intentional-error'; message: string }
  | { kind: 'communication-error'; message: string };

@Injectable({ providedIn: 'root' })
export class BridgeService {
  async check(fail: boolean): Promise<BridgeResult> {
    try {
      const message = await invoke<string>('bridge_check', { fail });
      return { kind: 'success', message };
    } catch (error) {
      if (typeof error === 'string' && error.startsWith(INTENTIONAL_FAILURE_PREFIX)) {
        return { kind: 'intentional-error', message: error.slice(INTENTIONAL_FAILURE_PREFIX.length) };
      }
      return { kind: 'communication-error', message: describeError(error) };
    }
  }
}

function describeError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}
