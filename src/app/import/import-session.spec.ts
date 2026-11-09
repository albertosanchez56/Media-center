import { TestBed } from '@angular/core/testing';
import { FileDialog } from './file-dialog';
import { ImportSession } from './import-session';

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

describe('ImportSession', () => {
  let dialog: FakeDialog;
  let session: ImportSession;

  beforeEach(() => {
    dialog = new FakeDialog();
    TestBed.configureTestingModule({ providers: [{ provide: FileDialog, useValue: dialog }] });
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
});
