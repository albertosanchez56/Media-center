import { LibraryId } from '../navigation/navigation';
import { describeError } from '../shared/describe-error';

/** Selección en memoria. F3.2 la recibirá; esta tarea no la guarda. */
export interface ImportSelection {
  library: LibraryId;
  kind: 'files' | 'folder';
  paths: readonly string[];
}

export function normalizePaths(result: string | string[] | null | undefined): string[] | null {
  if (result == null) {
    return null;
  }
  const paths = (Array.isArray(result) ? result : [result])
    .map((path) => path.trim())
    .filter((path) => path.length > 0);
  return paths.length === 0 ? null : paths;
}

/** Una cancelación o una lista vacía conservan la selección anterior. */
export function applyDialogResult(
  current: ImportSelection | null,
  library: LibraryId,
  kind: ImportSelection['kind'],
  result: string | string[] | null | undefined,
): ImportSelection | null {
  const paths = normalizePaths(result);
  if (paths === null) {
    return current;
  }
  if (kind === 'folder') {
    return { library, kind, paths: [paths[0]] };
  }
  return { library, kind, paths };
}

export function withLibrary(
  selection: ImportSelection | null,
  library: LibraryId,
): ImportSelection | null {
  if (selection === null || selection.library === library) {
    return selection;
  }
  return { ...selection, library };
}

export function pathName(path: string): string {
  const trimmed = path.trim().replace(/[\\/]+$/, '');
  const slash = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'));
  const name = slash === -1 ? trimmed : trimmed.slice(slash + 1);
  return name.length > 0 ? name : path.trim();
}

export function dialogFailure(error: unknown): string {
  const detail = describeError(error).trim();
  if (detail.length === 0 || detail === 'undefined' || detail === 'null') {
    return 'No se ha podido abrir el diálogo de selección.';
  }
  return `No se ha podido abrir el diálogo de selección. ${detail}`;
}
