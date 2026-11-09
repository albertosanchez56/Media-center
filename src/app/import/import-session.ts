import { Injectable, inject, signal } from '@angular/core';
import { LibraryId } from '../navigation/navigation';
import { FileDialog } from './file-dialog';
import {
  ImportSelection,
  applyDialogResult,
  dialogFailure,
  withLibrary,
} from './import-selection';

@Injectable({ providedIn: 'root' })
export class ImportSession {
  private readonly dialog = inject(FileDialog);
  private libraryChosen = false;

  readonly visible = signal(false);
  readonly pending = signal(false);
  readonly library = signal<LibraryId>('music');
  readonly selection = signal<ImportSelection | null>(null);
  readonly error = signal<string | null>(null);

  show(section: LibraryId): void {
    if (!this.libraryChosen && this.selection() === null) {
      this.library.set(section);
    }
    this.visible.set(true);
  }

  chooseLibrary(library: LibraryId): void {
    this.libraryChosen = true;
    this.library.set(library);
    this.selection.update((current) => withLibrary(current, library));
  }

  hide(): void {
    this.visible.set(false);
  }

  async pick(kind: ImportSelection['kind']): Promise<void> {
    if (this.pending()) {
      return;
    }
    this.pending.set(true);
    this.error.set(null);
    try {
      const result = kind === 'files' ? await this.dialog.openFiles() : await this.dialog.openFolder();
      this.selection.set(applyDialogResult(this.selection(), this.library(), kind, result));
    } catch (error) {
      this.error.set(dialogFailure(error));
    } finally {
      this.pending.set(false);
    }
  }
}
