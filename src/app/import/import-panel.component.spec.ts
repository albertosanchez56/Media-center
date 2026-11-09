import { TestBed } from '@angular/core/testing';
import { FileDialog } from './file-dialog';
import { ImportPanelComponent } from './import-panel.component';
import { ImportSession } from './import-session';
import { ImportTransport } from './import-transport';

describe('ImportPanelComponent', () => {
  it('says that subfolders are not imported and that the decision is still open', async () => {
    await TestBed.configureTestingModule({
      imports: [ImportPanelComponent],
      providers: [
        {
          provide: FileDialog,
          useValue: {
            openFiles: () => Promise.resolve(null),
            openFolder: () => Promise.resolve('C:\\Musica'),
          },
        },
        {
          provide: ImportTransport,
          useValue: {
            folderStatus: () => Promise.resolve({ path: 'C:\\Biblioteca', state: 'proposed' }),
            start: () => Promise.resolve(),
            cancel: () => Promise.resolve(),
            listen: () => Promise.resolve(() => undefined),
          },
        },
      ],
    }).compileComponents();

    const fixture = TestBed.createComponent(ImportPanelComponent);
    const session = TestBed.inject(ImportSession);
    session.show('music');
    await session.pick('folder');
    await fixture.whenStable();

    const text = (fixture.nativeElement as HTMLElement).textContent ?? '';
    expect(text).toContain('Solo se importan los archivos de esta carpeta. Las subcarpetas no se recorren.');
    expect(text).toContain('La importación de subcarpetas está pendiente de decidir.');
    expect(text).toContain('Copiar');
    expect(text).toContain('Mover');
  });
});
