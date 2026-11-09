import { LibraryId } from '../navigation/navigation';
import {
  ImportSelection,
  applyDialogResult,
  dialogFailure,
  normalizePaths,
  pathName,
  withLibrary,
} from './import-selection';

const mp4 = 'C:\\Música con espacio\\clip.mp4';

describe('import selection', () => {
  it('treats cancel and an empty list as no new selection', () => {
    expect(normalizePaths(null)).toBeNull();
    expect(normalizePaths(undefined)).toBeNull();
    expect(normalizePaths([])).toBeNull();
    expect(normalizePaths(['', '  '])).toBeNull();
    expect(normalizePaths('   ')).toBeNull();
  });

  it('keeps the previous selection when the dialog is cancelled', () => {
    const current: ImportSelection = { library: 'music', kind: 'files', paths: [mp4] };
    expect(applyDialogResult(current, 'video', 'folder', null)).toBe(current);
    expect(applyDialogResult(current, 'video', 'files', [])).toBe(current);
    expect(applyDialogResult(null, 'music', 'files', null)).toBeNull();
  });

  it('replaces the previous selection only with a valid one', () => {
    const current: ImportSelection = { library: 'music', kind: 'files', paths: [mp4] };
    const folder = 'D:\\Películas\\Largas';
    expect(applyDialogResult(current, 'movie', 'folder', folder)).toEqual({
      library: 'movie',
      kind: 'folder',
      paths: [folder],
    });
    expect(applyDialogResult(current, 'video', 'files', ['a.mp4', 'b.flac'])).toEqual({
      library: 'video',
      kind: 'files',
      paths: ['a.mp4', 'b.flac'],
    });
  });

  it('accepts the same mp4 in any library without looking at the extension', () => {
    const libraries: LibraryId[] = ['music', 'video', 'movie'];
    for (const library of libraries) {
      expect(applyDialogResult(null, library, 'files', [mp4])).toEqual({
        library,
        kind: 'files',
        paths: [mp4],
      });
    }
    const selected = applyDialogResult(null, 'music', 'files', [mp4]);
    expect(withLibrary(selected, 'movie')).toEqual({
      library: 'movie',
      kind: 'files',
      paths: [mp4],
    });
  });

  it('shows the last path segment for names with spaces and long folders', () => {
    const long = 'C:\\Users\\Alguien\\Biblioteca muy larga\\Subcarpeta\\vídeo final.mp4';
    expect(pathName(long)).toBe('vídeo final.mp4');
    expect(pathName('D:/Películas/')).toBe('Películas');
    expect(pathName(long)).not.toBe(long);
  });

  it('explains a dialog failure without dropping the detail', () => {
    expect(dialogFailure(new Error('permiso denegado'))).toBe(
      'No se ha podido abrir el diálogo de selección. permiso denegado',
    );
    expect(dialogFailure('')).toBe('No se ha podido abrir el diálogo de selección.');
  });
});
