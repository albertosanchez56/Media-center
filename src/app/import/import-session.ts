import { Injectable, inject, signal } from '@angular/core';
import { LibraryId } from '../navigation/navigation';
import { describeError } from '../shared/describe-error';
import { FileDialog } from './file-dialog';
import {
  ImportSelection,
  applyDialogResult,
  dialogFailure,
  withLibrary,
} from './import-selection';
import { ImportItem, ImportProgress, ImportTransport } from './import-transport';

@Injectable({ providedIn: 'root' })
export class ImportSession {
  private readonly dialog = inject(FileDialog);
  private readonly transport = inject(ImportTransport);
  private libraryChosen = false;
  private listening = false;

  readonly visible = signal(false);
  readonly pending = signal(false);
  readonly running = signal(false);
  readonly library = signal<LibraryId>('music');
  readonly selection = signal<ImportSelection | null>(null);
  readonly error = signal<string | null>(null);
  readonly folder = signal<{ path: string; state: string } | null>(null);
  readonly results = signal<ImportItem[]>([]);

  show(section: LibraryId): void {
    if (!this.libraryChosen && this.selection() === null) {
      this.library.set(section);
    }
    this.visible.set(true);
    void this.refreshFolder();
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
    if (this.pending() || this.running()) {
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

  copy(): Promise<void> {
    return this.start('copy');
  }

  move(): Promise<void> {
    return this.start('move');
  }

  async cancel(): Promise<void> {
    await this.transport.cancel();
  }

  private async refreshFolder(): Promise<void> {
    try {
      this.folder.set(await this.transport.folderStatus());
    } catch (error) {
      if (this.error() === null) {
        this.error.set(describeError(error));
      }
    }
  }

  private async start(mode: 'copy' | 'move'): Promise<void> {
    const selection = this.selection();
    if (!selection || this.running() || this.pending()) {
      return;
    }
    this.running.set(true);
    this.error.set(null);
    this.results.set([]);
    try {
      await this.ensureListening();
      await this.transport.start({
        library: selection.library,
        mode,
        kind: selection.kind,
        paths: [...selection.paths],
      });
    } catch (error) {
      this.running.set(false);
      this.error.set(describeError(error));
    }
  }

  private async ensureListening(): Promise<void> {
    if (this.listening) {
      return;
    }
    await this.transport.listen((progress) => this.apply(progress));
    this.listening = true;
  }

  private apply(progress: ImportProgress): void {
    this.results.set(progress.items);
    if (!progress.finished) {
      return;
    }
    this.running.set(false);
    void this.refreshFolder();
    const pending = progress.items.filter((item) =>
      item.state === 'failed' || item.state === 'needs_review' || item.state === 'ready_for_db',
    );
    if (pending.length === 0) {
      this.selection.set(null);
      return;
    }
    const current = this.selection();
    if (current) {
      this.selection.set({
        ...current,
        paths: pending.map((item) => item.source).filter((path) => path.length > 0),
      });
    }
  }
}
