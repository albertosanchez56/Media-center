import { TestBed } from '@angular/core/testing';
import { FileDialog } from './file-dialog';
import { ImportSession } from './import-session';
import { ImportProgress, ImportRequest, ImportTransport } from './import-transport';

class FakeDialog extends FileDialog {
  filesResult: string | string[] | null = null;
  folderResult: string | string[] | null = null;
  fail: unknown = null;
  fileCalls = 0;
  folderCalls = 0;

  openFiles(): Promise<string | string[] | null> {
    this.fileCalls += 1;
    if (this.fail) {
      return Promise.reject(this.fail);
    }
    return Promise.resolve(this.filesResult);
  }

  openFolder(): Promise<string | string[] | null> {
    this.folderCalls += 1;
    if (this.fail) {
      return Promise.reject(this.fail);
    }
    return Promise.resolve(this.folderResult);
  }
}

class FakeTransport extends ImportTransport {
  request: ImportRequest | null = null;
  cancelCalls = 0;
  startError: unknown = null;
  private handler: ((progress: ImportProgress) => void) | null = null;

  folderReads = 0;

  folderStatus() {
    this.folderReads += 1;
    return Promise.resolve({ path: 'C:\\Biblioteca', state: 'proposed' as const });
  }

  start(request: ImportRequest) {
    this.request = request;
    if (this.startError) {
      return Promise.reject(this.startError);
    }
    return Promise.resolve();
  }

  cancel() {
    this.cancelCalls += 1;
    return Promise.resolve();
  }

  listen(handler: (progress: ImportProgress) => void) {
    this.handler = handler;
    return Promise.resolve(() => undefined);
  }

  emit(progress: ImportProgress): void {
    this.handler?.(progress);
  }
}

describe('ImportSession', () => {
  let dialog: FakeDialog;
  let transport: FakeTransport;
  let session: ImportSession;

  beforeEach(() => {
    dialog = new FakeDialog();
    transport = new FakeTransport();
    TestBed.configureTestingModule({
      providers: [
        { provide: FileDialog, useValue: dialog },
        { provide: ImportTransport, useValue: transport },
      ],
    });
    session = TestBed.inject(ImportSession);
  });

  it('keeps an empty selection when the first dialog is cancelled', async () => {
    session.show('video');
    dialog.filesResult = null;
    await session.pick('files');
    expect(session.selection()).toBeNull();
    expect(session.library()).toBe('video');
  });

  it('keeps the previous mp4 selection when a later dialog is cancelled or fails', async () => {
    session.show('music');
    dialog.filesResult = ['C:\\clips\\uno.mp4', 'C:\\clips\\dos.mp4'];
    await session.pick('files');
    dialog.filesResult = null;
    await session.pick('files');
    expect(session.selection()?.paths).toEqual(['C:\\clips\\uno.mp4', 'C:\\clips\\dos.mp4']);

    dialog.fail = new Error('cerrado');
    dialog.folderResult = 'C:\\otra';
    await session.pick('folder');
    expect(session.selection()?.kind).toBe('files');
    expect(session.error()).toContain('No se ha podido abrir el diálogo de selección.');
    expect(session.error()).toContain('cerrado');
  });

  it('lets the same mp4 belong to each library and ignores the visible section afterwards', async () => {
    session.show('music');
    dialog.filesResult = 'C:\\clips\\tema.mp4';
    await session.pick('files');
    session.chooseLibrary('video');
    expect(session.selection()).toEqual({
      library: 'video',
      kind: 'files',
      paths: ['C:\\clips\\tema.mp4'],
    });
    session.show('movie');
    expect(session.library()).toBe('video');
    session.chooseLibrary('movie');
    expect(session.selection()?.library).toBe('movie');
    expect(session.selection()?.paths).toEqual(['C:\\clips\\tema.mp4']);
  });

  it('replaces files with one folder and does not keep the old paths', async () => {
    dialog.filesResult = ['C:\\a.mp4'];
    await session.pick('files');
    dialog.folderResult = ['D:\\Colección', 'D:\\ignorada'];
    await session.pick('folder');
    expect(session.selection()).toEqual({
      library: 'music',
      kind: 'folder',
      paths: ['D:\\Colección'],
    });
  });

  it('sends the chosen library and keeps the selection when the command fails', async () => {
    session.show('music');
    dialog.filesResult = 'C:\\clips\\tema.mp4';
    await session.pick('files');
    session.chooseLibrary('movie');
    transport.startError = new Error('ocupada');
    await session.copy();
    expect(transport.request).toEqual({
      library: 'movie',
      mode: 'copy',
      kind: 'files',
      paths: ['C:\\clips\\tema.mp4'],
    });
    expect(session.selection()?.paths).toEqual(['C:\\clips\\tema.mp4']);
    expect(session.running()).toBe(false);
    expect(session.error()).toContain('ocupada');
  });

  it('shows each file state and keeps only the failed path', async () => {
    dialog.filesResult = ['C:\\uno.mp4', 'C:\\dos.mp4'];
    await session.pick('files');
    await session.copy();
    expect(session.running()).toBe(true);
    transport.emit({
      finished: false,
      items: [{ source: 'C:\\uno.mp4', state: 'transferring', message: 'Copiando' }],
    });
    expect(session.results()[0].state).toBe('transferring');
    transport.emit({
      finished: true,
      items: [
        { source: 'C:\\uno.mp4', state: 'completed', message: 'Lista' },
        { source: 'C:\\dos.mp4', state: 'failed', message: 'No cabe' },
      ],
    });
    expect(session.running()).toBe(false);
    expect(session.selection()?.paths).toEqual(['C:\\dos.mp4']);
  });

  it('rereads the managed folder once the import finishes', async () => {
    session.show('music');
    await Promise.resolve();
    const before = transport.folderReads;
    dialog.filesResult = 'C:\\uno.mp4';
    await session.pick('files');
    await session.copy();
    transport.emit({
      finished: true,
      items: [{ source: 'C:\\uno.mp4', state: 'completed', message: 'Lista' }],
    });
    await Promise.resolve();
    expect(transport.folderReads).toBe(before + 1);
    expect(session.selection()).toBeNull();
  });

  it('asks to cancel and does not request a delete', async () => {
    dialog.filesResult = 'C:\\uno.mp4';
    await session.pick('files');
    await session.move();
    await session.cancel();
    expect(transport.cancelCalls).toBe(1);
    expect(transport.request?.mode).toBe('move');
    expect(session.selection()?.paths).toEqual(['C:\\uno.mp4']);
  });
});
